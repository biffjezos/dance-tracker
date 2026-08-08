<!-- Handoff snapshot, delivered 2026-08-07 as SPECwebgpuoperations.md's
     precondition/dependency (referenced throughout that spec as
     "SPEC-webgpu-compute-backend.md"). Already implemented and merged to
     dev (Phase 0 landed - gpu module, Context.gpu/App.gpu). Included here
     for reference only, not as new work. Canonical/owned copy:
     .agents/roles/software_architect/docs/specifications/SPECwebgpucomputebackend-1.md
     (Software Architect). -->

<!-- Corrected 2026-08-08: see this file's own "Correction — 2026-08-08" section
     near the end, and RFC-006. -->


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
