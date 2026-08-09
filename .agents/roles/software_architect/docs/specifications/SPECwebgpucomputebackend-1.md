# Specification: WebGPU compute backend

**Precondition:** RFC-001 has landed. Before starting this spec's Phase 0, verify with `grep -rE "wgpu|ComputeBackend|GpuBackend|GpuContext|compute_mode" engine/src ui/scripts` — it must return zero matches. This spec builds fresh on a clean tree; it does not patch the removed attempt.

**Scope of this spec: the foundation only — no operation's own GPU work lives here.** This spec builds the `gpu` module, the `Context`/`App` wiring, and the async-safe dispatch mechanism every GPU-capable operation will use. It ends with a mechanism that compiles, boots safely, and does nothing operation-specific yet. Every actual operation (starting with `BLUR`) is specified separately in the companion document, `SPEC-webgpu-operations.md`, which depends on this one landing first. Selection stays automatic (GPU when available, CPU otherwise) — **no user-facing COMPUTE MODE switch in this round**, deferred; see "Out of scope."

## The core problem this spec has to solve, and why the previous attempt couldn't

WebGPU buffer readback is *unavoidably asynchronous* in a browser — there is no blocking-wait available, unlike native wgpu backends (Vulkan/Metal/DX12), which is what the removed attempt actually used. `Operation::execute()` is, and stays, fully synchronous — rewriting the whole execution engine (`RenderExecutor`, `PreviewExecutor`, every operation, the WASM boundary, the JS render loop) to be async is a categorically bigger change than this round asks for, and isn't necessary.

**Decision: GPU-backed operations use one-tick-latency pipelined dispatch, not same-tick blocking reads.** On a given tick, a GPU-backed operation returns the most recently *completed* GPU result (or falls back to CPU if none exists yet), and separately kicks off a fresh async GPU job for the current inputs, to be consumed on some future tick. This is a standard pattern for integrating async GPU work into a synchronous per-frame loop, and it fits precedent already in this codebase:

- `RenderExecutor`'s own cross-tick caching already treats "reuse a previous tick's result" as a first-class, correct behavior, not a compromise.
- `Ghost`'s `history: RefCell<VecDeque<Vec<f32>>>` (`operations/generators/ghost.rs`) is the exact existing idiom for "an operation holds its own private, interior-mutable, per-tick state" — the pending/last-completed GPU job state below follows the same shape.

The tradeoff, stated plainly: a GPU-backed operation's output can lag its true inputs by up to one tick while a new GPU result is in flight. At real-time framerates this is imperceptible (a fraction of the frame budget), and it's a smaller cost than either blocking the browser's main thread (impossible) or rearchitecting the whole engine to be async (out of proportion to what was asked for).

## Design

### Confirmed dependency versions

Re-add to `engine/Cargo.toml`: `wgpu = "30.0.0"`, `pollster = "1.0.1"`, `bytemuck = "1.25.2"` — this exact combination is confirmed compatible with the currently-pinned `wasm-bindgen = "0.2.126"`/`web-sys = "0.3.103"`: the removed attempt used it, and its `cargo build` demonstrably progressed past dependency resolution (it failed on ordinary Rust type errors in our own code, `E0277`/`E0599`, which can only happen after crates resolve successfully). This isn't a claim that these are the latest or most feature-complete versions available — only that they're a verified-safe starting point. **`wasm-bindgen-futures = "0.4.76"` is already a dependency** (predates this work) — reuse it, don't add a second pin. After re-adding, run `cargo tree -p wgpu` and pin whatever the lockfile actually resolves to, rather than leaving loose ranges a future `cargo update` could silently drift away from this verified combination.

### `gpu` module — the actual abstraction layer between app and GPU

This is the reusable boundary the user asked for: everything that knows about wgpu/WebGPU specifics lives here, once, so a GPU-capable operation never has to touch adapter/device/queue setup itself.

- `engine/src/gpu/mod.rs`: `GpuState { device: wgpu::Device, queue: wgpu::Queue }`, built once via `GpuState::new().await -> Result<Self, String>` (adapter/device request — this is the one place `wgpu::Instance`/`request_adapter`/`request_device` are called).
- `GpuState` provides small, generic, reusable helpers any GPU-capable operation builds on: `create_shader(wgsl: &str) -> ShaderModule`, `create_compute_pipeline(...)`, and buffer helpers for upload/dispatch/readback — but **no operation-specific method** (no `GpuState::blur()`). This is the fix for the removed attempt's core mistake: the shared layer stays generic; each operation owns its own pipeline, shader, and dispatch logic on top of it.
- `Context` (`compositor/context.rs`) gains `pub gpu: Option<Arc<GpuState>>` — **`Option`, not a bare trait object**, so `#[derive(Default)]` on `Context` is trivially preserved (`None` is a perfectly good default, and it *is* the correct default: "GPU not yet available"). This single choice is what directly implements "GPU first, CPU fallback" without needing a `ComputeMode` enum or any mode-selection UI at all — `None` means CPU, `Some` means GPU is available to try.

### App boot: guarded, non-blocking, never fails startup

- `App::init_gpu()` stays `async`, but the JS call site (`ui/scripts/app.js`'s `boot()`) must **not** `await` it inline before the rest of boot — kick it off via `wasm_bindgen_futures::spawn_local` (or the JS-side equivalent, a fire-and-forget `.then()`/`.catch()`) so a GPU init failure or a browser with no WebGPU support **never blocks or breaks app startup**. The app must render and be fully usable on CPU immediately; `App.gpu` (and downstream, `Context.gpu`) simply stays `None` until (and unless) GPU init resolves successfully in the background. This directly fixes the removed attempt's most severe bug (`await wasmApp.init_gpu()` with no guard breaking boot entirely on unsupported machines).

**The exact `App` → `Context` wiring seam, named precisely so it isn't missed:** `App::context(&self, preview: bool) -> Context` (`engine/src/app.rs`, private, currently ~line 517) is the single place both `render_tick` and `preview_tick` build a fresh `Context` each call — it already assembles `meta`, `resources`, and `input_bboxes` there. Add `gpu: self.gpu.clone()` to that same struct literal. Add `App.gpu: Option<Arc<GpuState>>` as a new field, set once `init_gpu()`'s spawned task resolves.

**Explicitly do not do what the removed attempt did here:** its version of this exact function had `compute: self.compute.clone().expect("Compute backend not initialized")` — an `.expect()` that panics on *every* render tick until GPU init resolves (which may be never, or may race the first tick). `App.gpu` must be read with a plain `.clone()`; `None` is an expected, common, non-error state, not something to unwrap past.

**Correction — 2026-08-08: `App.gpu: Option<Arc<GpuState>>` as a plain
field on `App` violates this section's own "never blocks or breaks app
startup" claim.** Found via a Management bug report (app permanently
stops responding to any interaction after a WebGPU init failure). Full
detail: RFC-007 (`.agents/communication/rfc/RFC007initgpuborrowlockup.md`).
Summary: `init_gpu(&mut self)`'s `async fn` desugaring holds a
`wasm-bindgen`-guarded mutable borrow of `self` for its entire body,
including across the `.await` on `GpuState::new()`. A raw JS exception
escaping mid-poll there (confirmed: a browser adapter/device request that
throws directly from generated glue rather than resolving to a Rust
`Result::Err`) bypasses the normal poll-return path that releases that
borrow — leaving it stuck "checked out" for the rest of the page's life,
which then makes `wasm-bindgen` correctly refuse every subsequent call
into *any* method on the same `App` object, not just GPU-related ones.
Corrected design: `App.gpu` must be `Rc<RefCell<Option<Arc<GpuState>>>>`,
not a plain field — mirroring the `Rc<RefCell<...>>>` interior-mutability
pattern every GPU-backed operation already uses for its own cross-tick
state (`pending`/`last_gpu_result`). `init_gpu` clones the `Rc` out and
does its actual async work against that clone, never against `&mut self`
directly, so no failure inside the `.await` — clean `Err` or raw JS throw
alike — can ever leave a `wasm-bindgen` object-borrow stuck. See RFC-007
for the full required change and acceptance criteria.

### The pattern every GPU-backed operation follows

This is the reusable recipe — every operation specified in `SPEC-webgpu-operations.md` (and any added after it) implements this shape rather than re-deriving it. Named generically here (`Op`/`PendingJob`/`CompletedJob`) since these types are **operation-owned, not shared** — each operation defines its own, in its own file, next to its own struct, matching `Ghost`'s interior-mutability idiom (`RefCell<VecDeque<...>>` in `operations/generators/ghost.rs`):

```rust
pub struct Op {
    // ...the operation's own ordinary parameters...
    gpu_pipeline: RefCell<Option<OpGpuPipeline>>,   // lazily built the first time ctx.gpu is Some
    pending: RefCell<Option<PendingOpJob>>,          // an in-flight async dispatch, if any
    last_gpu_result: RefCell<Option<CompletedOpJob>>, // most recent completed result + the fingerprint it was computed from
}
```

**Fingerprint, precisely — this part is not operation-specific, use it exactly as described for every operation:** capture the fingerprint from the *wired* `Value` (via `find_input`), **before** calling `FloatImage::from_value` on it — `from_value` clones the `FloatImage` out of its `Arc` (`Ok((**float_image).clone())` in `graphics/float_image.rs`), so anything derived from *after* that call has no pointer identity back to the original. Compare fingerprints via `crate::compositor::value::value_ptr_eq(&new, &old)` — the exact function `RenderExecutor`'s own cross-tick cache already uses for identical reasons. `Value::clone()` on the Arc-wrapped variants is a cheap `Arc::clone`, not a pixel copy. Include every parameter the operation's own math actually depends on (a radius, an offset, a threshold — whatever's relevant) alongside the source value(s).

**Dispatch, every tick:**
1. `ctx.gpu.is_none()` → CPU fallback, unconditionally. Always correct, always available.
2. `ctx.gpu.is_some()`:
   - If `last_gpu_result`'s fingerprint matches this tick's actual fingerprint → use it.
   - Otherwise → CPU fallback **for this tick only** (never block waiting for GPU), and (re-)kick off a fresh async dispatch for the current fingerprint via `wasm_bindgen_futures::spawn_local`, writing into `pending`/`last_gpu_result` when it resolves.

**Target-conditional dispatch, not two designs:** `#[cfg(target_arch = "wasm32")]` uses the non-blocking `map_async` + `spawn_local` pipelined path above. `#[cfg(not(target_arch = "wasm32"))]` (native, i.e. `cargo test`) may use a blocking read (`pollster::block_on` or `device.poll(Wait)`) — native backends genuinely support blocking, so this isn't wrong there, only in the browser. This split is exactly the existing pattern `profiling.rs`'s `measure_ms` already uses (`#[cfg(target_arch = "wasm32")]` vs. not) — follow that precedent, don't invent a new one per operation. This is also what makes every GPU path deterministically testable under `cargo test`, without mocking wgpu away.

**Required correctness detail — every GPU-backed operation needs this, not just the first one:** `RenderExecutor`'s cross-tick cache (`compositor/executors/render.rs`) skips calling `execute()` again at all for a node whose parameters and resolved inputs haven't changed. A GPU-backed operation on a genuinely static upstream graph would therefore only ever call `execute()` once — if that single call fell back to CPU (no GPU result ready yet), it would **never get a chance to pick up the completed GPU result later**, staying stuck on CPU forever even after the async job finishes. Fix: `is_live()` must return `true` whenever `pending.borrow().is_some()` — forces re-execution on the next tick specifically so the operation gets a chance to consume a just-completed result and update its cache, returning to `false`-driven caching once stable. `is_live()` already takes `&self`, so reading this from the same `RefCell` is a direct, no-new-mechanism fix. **Every per-operation phase in `SPEC-webgpu-operations.md` must include a regression test for this specific failure mode** — it's easy to miss per-operation, not just once.

**Numerical tolerance:** GPU (WGSL) float math and CPU (Rust) float math for the same operation will not always produce bit-identical results (different summation order, different intermediate rounding). Tests comparing GPU output to CPU output must use a small per-channel tolerance (e.g. `(a - b).abs() < 1e-4`), not exact equality — unlike the CPU-only consume-equivalence tests elsewhere in the bbox work, which are exact.

## Phased work

**Phase 0 — `gpu` module foundation.** `GpuState`, generic shader/buffer helpers, `Context.gpu: Option<Arc<GpuState>>`, `App.gpu: Option<Arc<GpuState>>`, guarded non-blocking boot-time init, and the `App::context()` wiring seam above. No operation uses it yet. Acceptance: app boots identically with or without WebGPU support in the browser (test both); `Context` still derives `Default`; full existing test suite unaffected; **and** — the seam itself, not just its prerequisites — after `init_gpu()` resolves in a test/integration context, the *next* `App::context()` call returns `Context.gpu = Some(...)`, proving a resolved GPU handle actually becomes reachable from a live render tick rather than silently stuck at `None`.

This is the only phase in this spec. Every operation's own GPU work is specified in `SPEC-webgpu-operations.md`, which depends on this phase landing first.

## Out of scope for this round

- **COMPUTE MODE user-facing switch** (CPU/GPU/AUTO selection UI). `Context.gpu: Option<...>` already *is* the AUTO behavior (GPU when available, CPU otherwise) with no user control needed. A manual override is real future work, not needed now — see the companion menu specification for where such a control would eventually register, if wanted.
- **Any operation-specific GPU work at all** — see `SPEC-webgpu-operations.md`. This spec's job is to make the foundation exist and be provably safe; it does not accelerate anything yet.
- **Buffer-shrinking / the RAM-reduction backlog item.** Unrelated axis of work, already tracked separately in `PARKED_WORK.md`.

## Acceptance criteria (whole spec)

1. `cargo build`/`cargo test` clean on native.
2. App boots and is fully usable on a machine with no WebGPU support at all, with zero errors surfaced to the user.
3. `Context` and `App` both carry a working, correctly-wired `gpu: Option<Arc<GpuState>>` (per the `App::context()` seam above), even though nothing consumes it yet.
4. No blocking call reachable from the `wasm32` build target anywhere in the `gpu` module — audit every `wgpu` call and confirm no `PollType::Wait`/blocking `recv()` exists outside a `#[cfg(not(target_arch = "wasm32"))]` branch.
5. `SPEC-webgpu-operations.md` can be started immediately after this lands, with no further foundation work needed first.

## Correction — 2026-08-08: unbounded concurrent dispatch, and readback panics on failure

Filed against this spec's own "Dispatch, every tick" and "The pattern every
GPU-backed operation follows" sections above (not a new design — a
correction to two real defects in the design as written, found via a
Management bug report: constant high GPU/fan load and intermittent full
app hangs since this backend shipped). Full detail, root-cause trace, and
required fix: RFC-006 (`.agents/communication/rfc/RFC006gpudispatchoverloadandpanic.md`).
Summary of what was wrong in the two sections above, for anyone reading
this spec fresh:

**Defect 1 — "Dispatch, every tick" step 2's re-dispatch condition is
wrong.** As written and as implemented in every operation
(`already_pending = pending.borrow().as_ref().is_some_and(|p|
p.matches(&fingerprint))`), a new dispatch is gated on whether *this exact
fingerprint* is already in flight — not on whether *any* dispatch is in
flight. For an input that changes every tick (any live video source, or
any procedural generator with a time-varying parameter — this app's
primary use case, not an edge case), the fingerprint never repeats, so
this condition is always false: a brand new GPU dispatch (fresh buffer
allocations, a fresh `spawn_local` task) launches on literally every
animation frame, with no cap on how many can be in flight simultaneously,
none of which ever get consumed before being superseded by the next
tick's fingerprint change (WebGPU buffer-mapping readback takes multiple
frames; the one-tick-latency assumption this spec's own "Decision" section
states does not hold for continuously-changing input). Corrected rule:
gate on **any** pending dispatch for this operation instance, not a
fingerprint match — `let has_pending = self.pending.borrow().is_some();
if !has_pending { dispatch_gpu(...) }`. This bounds concurrent in-flight
GPU work per operation to exactly one dispatch, regardless of how fast
the input changes; content that changes faster than one dispatch's
readback latency now correctly stays on CPU fallback throughout (no
lag, no wasted GPU work), same as intended, while content stable across
multiple ticks still benefits from the GPU result once it lands.

**Defect 2 — `read_buffer_blocking`/`read_buffer_async` (`gpu/mod.rs`)
violate this module's own stated contract.** `GpuState::new()`'s doc
comment states adapter/device request "Resolves to `Err` — never panics."
The same contract was never extended to buffer mapping: both readback
functions `.expect()` on a mapping failure. On `wasm32`, `read_buffer_async`
runs inside a detached `wasm_bindgen_futures::spawn_local` task with no
caller able to catch a panic there — a real mapping failure (e.g. GPU
device lost, plausible after Defect 1's sustained overload, or from
ordinary thermal/driver conditions independent of it) traps the entire
WASM instance with no recovery path: every subsequent exported call
becomes unreliable for the rest of the session. Corrected rule: mapping
failure must resolve to `Result`/`Option`, propagated back through the
operation's spawned task to clear `pending` and leave `last_gpu_result`
untouched (silently continuing on CPU fallback), never a panic.

RFC-006 has the full required-change list (all 16 operations sharing this
pattern) and acceptance criteria.

## Correction — 2026-08-09: adapter/device request throws an uncaught JS error instead of resolving `Err`, and this spec's own AC2 was never actually met

Filed against this spec's "`gpu` module" section (line ~28: `GpuState::new()`
"this is the one place `wgpu::Instance`/`request_adapter`/`request_device`
are called") and this spec's own Acceptance Criterion 2 ("App boots and
is fully usable on a machine with no WebGPU support at all, with **zero
errors surfaced to the user**"). Management reported, again, on a build
that already includes RFC-007's fix:

```
dance_tracker_engine.js:623 Failed to create WebGPU Context Provider
dance_tracker_engine.js:627 Uncaught TypeError: Cannot read properties of null (reading 'requestDevice')
```

**Honest status, stated plainly per Management's standing instruction
against half-measures and quiet workarounds:** RFC-007 fixed the
*consequence* of this error (a `wasm-bindgen` object-borrow lockup that
bricked the whole app) — it explicitly never claimed to fix the error
itself, and said so in its own text. That was correct scoping for a
critical, app-breaking bug at the time, but it left this spec's own AC2
genuinely unmet, and left the real question — why does adapter/device
request fail *at all* on Management's machine — uninvestigated. This
correction closes that gap.

**What "Failed to create WebGPU Context Provider" tells us:** this is
`wgpu`'s own webgpu-backend log line, printed when
`navigator.gpu.requestAdapter()` resolves to `null`/`undefined` — i.e.
the browser itself is reporting no usable WebGPU adapter, not a
transient or ambiguous failure. `GpuState::new()`
(`engine/src/gpu/mod.rs`) already correctly awaits `instance.request_adapter(...)`
and maps a clean `Err` via `.map_err(...)`  — but this particular failure
mode never reaches that `Result` boundary at all: `wgpu` 30.0.0's webgpu
backend logs the failure and then still proceeds to call
`adapter.request_device(...)` on the `null` result internally, which
throws a raw, unguarded JS `TypeError` from generated glue
(`__wbg_requestDevice_...`) — bypassing `wgpu`'s own `Result`-returning
API surface entirely. This is best understood as an upstream `wgpu`
defect (or at minimum a null-adapter edge case its webgpu backend doesn't
handle as gracefully as its own logged message implies), not a mistake in
this codebase's own `GpuState::new()`, which does everything correctly
with the `Result` it's actually given the chance to see.

**Correction to this correction, same day, before RFC-010 even reached
implementation — recorded here rather than silently edited, per this
project's own rule that communication artifacts are immutable and
specifications must not be silently changed to match new information
without saying so:** the paragraph below originally listed "this
browser/machine has no WebGPU support at all" as a live, undetermined
possibility. Management has since confirmed WebGPU **was** working
earlier in this same investigation — real available-feature output in
the JS console, and the GPU genuinely under load (the fan noise that
started this entire investigation was, in part, real GPU work
happening). That directly rules out "never supported" as an explanation.
Scenario 1 below is kept in the numbered list for completeness (a real
category of failure this codebase must still handle honestly, in
general, for other users/machines) but is **not believed to be what's
happening on Management's own machine** — see Scenario 3, added below,
which fits the actual observed timeline.

**Three genuinely different underlying situations can produce the exact
same symptom. This codebase cannot currently tell them apart on its own
— this matters and must be determined, not assumed:**

1. **This specific browser/machine has no WebGPU support at all**
   (disabled, unsupported OS/GPU/driver combination, a remote/virtualized
   display, an older browser version, WebGPU behind a disabled flag —
   many real, common cases in general). In this scenario, **no code
   change in this repository can make real GPU compute happen** — this is
   a hard platform limitation, not a bug this team can fix. The correct,
   complete deliverable here is: no uncaught error, a clean internal
   `Err`, and clear, honest, visible confirmation that the app is running
   on CPU — not a promise that GPU will somehow start working. **Ruled
   out for Management's own machine** (see above) but must still be
   handled correctly for any other user/browser that genuinely lacks
   WebGPU — Part B's pre-flight check (below) handles this scenario
   regardless of which one turns out to be true here.
2. **`Cargo.toml`'s bare `wgpu = "30.0.0"` (no explicit `features = [...]`)
   resolves to a feature set that's missing something load-bearing** —
   e.g. a WebGL2-based fallback backend that a differently-configured
   build would have used instead of only attempting native WebGPU. This
   is genuinely fixable if true, but doesn't fit the observed timeline
   well either (a missing feature flag would have failed from the very
   first run, not after previously working) — kept as a secondary
   possibility, not the leading one.
3. **(Leading hypothesis, added after Management's clarification) The
   browser itself disabled/blocklisted WebGPU for this profile after
   repeated GPU-process instability, caused by the original (now-fixed
   by RFC-006) unbounded-dispatch overload bug.** Chromium-based browsers
   have a real, documented self-protection mechanism: after enough
   GPU-process crashes tied to a feature, that feature gets added to an
   internal workarounds/blocklist for the current profile and stops being
   offered — until the GPU cache is cleared, the blocklist resets, or the
   browser/profile is reset. This fits the actual timeline exactly: GPU
   worked and was overloaded (RFC-006's own root cause, very plausibly
   crashing the GPU process repeatedly under that load) → GPU now
   unavailable. **If this is the real cause, it may not require a code
   change to resolve at all** — check `chrome://gpu` (or the equivalent
   internals page for whatever browser Management is using) for WebGPU's
   listed status and any blocklist entries, and try clearing the GPU
   cache / fully restarting the browser, before assuming a code fix is
   required.

Whoever picks this up: **check Scenario 3 first** (fastest, no code
required, directly matches the reported timeline) before spending time on
Scenario 2's `cargo tree` investigation. Report which of the three is
real, with evidence — don't guess and don't silently pick one.

**Required change, regardless of which scenario turns out to be true —
this part is unconditional:**

Stop letting `wgpu`'s internal null-adapter path ever get reached at all.
Before calling `crate::gpu::GpuState::new()`, perform this codebase's
**own** pre-flight adapter check, using `web_sys`'s WebGPU bindings
directly (new `Cargo.toml` features required: `Gpu`, `GpuAdapter`, and
`Navigator` if not already implied by the existing `Window` feature):

1. `web_sys::window().navigator().gpu()` — if this is `undefined`/`None`,
   the browser has no WebGPU API at all. Return a clean, controlled
   `Err("WebGPU not supported by this browser")` immediately. Never call
   into `wgpu` at all in this case.
2. If `Navigator.gpu` exists, call `.request_adapter()` on it directly
   (via `wasm_bindgen_futures::JsFuture` wrapping the returned `Promise`,
   the same pattern `read_buffer_async` already uses for a different
   Promise) and check whether the resolved value is `null`. If so, return
   a clean, controlled `Err("No compatible GPU adapter available")`.
   Never call into `wgpu`'s `request_adapter`/`request_device` in this
   case either.
3. Only when this pre-flight check finds a genuine, non-null adapter,
   proceed to let `wgpu::Instance::request_adapter()`/`request_device()`
   run as they do today. Since this pre-check and `wgpu`'s own
   (separate) request happen on the same browser, same tick, a
   pre-confirmed-available adapter makes it overwhelmingly unlikely
   `wgpu`'s own internal call hits the null case at all.

This closes the actual reported symptom — the raw, uncaught,
developer-console-only `TypeError` — for **both** scenarios above: if
scenario 1 is real, the app now fails cleanly and says so, honestly, with
a real message this codebase controls, instead of a confusing browser
internals crash. If scenario 2 is real and gets fixed at the `Cargo.toml`
level, this pre-flight check still costs nothing and remains correct
defense-in-depth against any *other* browser where WebGPU genuinely isn't
available.

**Also required — status bar backend visibility, per Management's direct
request, and connecting to already-parked work rather than adding a new
disconneted field:** `PARKED_WORK.md` already has an open item (added
alongside RFC-008/SPEC-OUTPUT-RENDER-V1) noting the status bar's dead
`FPS` field is slated for removal. Reuse that freed slot — don't add a
new, ninth status bar element — to show the actual active compute
backend, live:

- New `App` method, e.g. `pub fn active_backend(&self) -> String`
  returning `"GPU"` if `self.gpu.borrow().is_some()`, `"CPU"` otherwise
  (reads the exact same `Rc<RefCell<Option<Arc<GpuState>>>>>` RFC-007
  already introduced — no new state).
  Note this reports "does at least one successfully-initialized GPU
  handle exist," not "is the GPU actively computing this exact tick" —
  precise enough to answer Management's actual question ("is WebGPU
  being used at all right now") without needing per-tick dispatch
  telemetry, which is a different, larger feature not asked for here.
- `ui/scripts/engine/render.js`'s `loop()` (already reads
  `wasmApp.is_output_out_of_gamut()` every tick, the exact same shape of
  call) reads this each tick and writes `"BACKEND: GPU"` /
  `"BACKEND: CPU"` into the status bar slot the `FPS` field's removal
  frees up.
- This must reflect reality honestly at every point in the app's
  lifecycle: `"BACKEND: CPU"` from boot until (and unless) `init_gpu()`'s
  pre-flight-checked negotiation actually succeeds — no premature "GPU"
  label before a real `GpuState` exists, no stale label if it later fails.

**What this correction does and does not promise, stated explicitly per
Management's own standing instruction against overclaiming:** this
guarantees no more uncaught console errors from adapter/device
negotiation, and guarantees the status bar always honestly reflects
whether GPU is actually active. It does **not** guarantee GPU will become
active on any specific machine — that depends entirely on which of
scenario 1 or 2 above is true, which is not yet known and must be
determined and reported as part of implementing this, not assumed away.

Related specifications: `SPEC-webgpu-operations.md` (every GPU-backed
operation reads the same `Context.gpu`, unaffected by this correction).
`SPEC-OUTPUT-RENDER-V1`'s parked status-bar cleanup item (`PARKED_WORK.md`)
is the origin of the freed `FPS` slot this correction reuses — implement
that field's removal as part of this work, not as a separate pass, since
they touch the same status bar element.

Full required-change detail and acceptance criteria: RFC-010
(`.agents/communication/rfc/RFC010webgpudiagnosticsandbackendindicator.md`).
