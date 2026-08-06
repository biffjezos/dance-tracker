# Specification: WebGPU compute backend

**Precondition:** RFC-001 has landed. Before starting this spec's Phase 0, verify with `grep -rE "wgpu|ComputeBackend|GpuBackend|GpuContext|compute_mode" engine/src ui/scripts` — it must return zero matches. This spec builds fresh on a clean tree; it does not patch the removed attempt.

**Scope for this round, per direction received:** a real, working GPU compute path for `BLUR` only, selected automatically (GPU when available, CPU otherwise) — **no user-facing COMPUTE MODE switch in this round.** That's explicitly deferred; see "Out of scope."

## The core problem this spec has to solve, and why the previous attempt couldn't

WebGPU buffer readback is *unavoidably asynchronous* in a browser — there is no blocking-wait available, unlike native wgpu backends (Vulkan/Metal/DX12), which is what the removed attempt actually used. `Operation::execute()` is, and stays, fully synchronous — rewriting the whole execution engine (`RenderExecutor`, `PreviewExecutor`, every operation, the WASM boundary, the JS render loop) to be async is a categorically bigger change than this round asks for, and isn't necessary.

**Decision: GPU-backed operations use one-tick-latency pipelined dispatch, not same-tick blocking reads.** On a given tick, a GPU-backed operation returns the most recently *completed* GPU result (or falls back to CPU if none exists yet), and separately kicks off a fresh async GPU job for the current inputs, to be consumed on some future tick. This is a standard pattern for integrating async GPU work into a synchronous per-frame loop, and it fits precedent already in this codebase:

- `RenderExecutor`'s own cross-tick caching already treats "reuse a previous tick's result" as a first-class, correct behavior, not a compromise.
- `Ghost`'s `history: RefCell<VecDeque<Vec<f32>>>` (`operations/generators/ghost.rs`) is the exact existing idiom for "an operation holds its own private, interior-mutable, per-tick state" — the pending/last-completed GPU job state below follows the same shape.

The tradeoff, stated plainly: a GPU-backed operation's output can lag its true inputs by up to one tick while a new GPU result is in flight. At real-time framerates this is imperceptible (a fraction of the frame budget), and it's a smaller cost than either blocking the browser's main thread (impossible) or rearchitecting the whole engine to be async (out of proportion to what was asked for).

## Design

### `gpu` module — the actual abstraction layer between app and GPU

This is the reusable boundary the user asked for: everything that knows about wgpu/WebGPU specifics lives here, once, so a GPU-capable operation never has to touch adapter/device/queue setup itself.

- `engine/src/gpu/mod.rs`: `GpuState { device: wgpu::Device, queue: wgpu::Queue }`, built once via `GpuState::new().await -> Result<Self, String>` (adapter/device request — this is the one place `wgpu::Instance`/`request_adapter`/`request_device` are called).
- `GpuState` provides small, generic, reusable helpers any GPU-capable operation builds on: `create_shader(wgsl: &str) -> ShaderModule`, `create_compute_pipeline(...)`, and buffer helpers for upload/dispatch/readback — but **no operation-specific method** (no `GpuState::blur()`). This is the fix for the removed attempt's core mistake: the shared layer stays generic; each operation owns its own pipeline, shader, and dispatch logic on top of it.
- `Context` (`compositor/context.rs`) gains `pub gpu: Option<Arc<GpuState>>` — **`Option`, not a bare trait object**, so `#[derive(Default)]` on `Context` is trivially preserved (`None` is a perfectly good default, and it *is* the correct default: "GPU not yet available"). This single choice is what directly implements "GPU first, CPU fallback" without needing a `ComputeMode` enum or any mode-selection UI at all — `None` means CPU, `Some` means GPU is available to try.

### App boot: guarded, non-blocking, never fails startup

- `App::init_gpu()` stays `async`, but the JS call site (`ui/scripts/app.js`'s `boot()`) must **not** `await` it inline before the rest of boot — kick it off via `wasm_bindgen_futures::spawn_local` (or the JS-side equivalent, a fire-and-forget `.then()`/`.catch()`) so a GPU init failure or a browser with no WebGPU support **never blocks or breaks app startup**. The app must render and be fully usable on CPU immediately; `App.gpu` (and downstream, `Context.gpu`) simply stays `None` until (and unless) GPU init resolves successfully in the background. This directly fixes the removed attempt's most severe bug (`await wasmApp.init_gpu()` with no guard breaking boot entirely on unsupported machines).

### `BLUR`'s GPU path

Add to `Blur` (`operations/transform/blur.rs`), matching `Ghost`'s interior-mutability idiom:

```rust
pub struct Blur {
    pub radius_px: u32,
    gpu_pipeline: RefCell<Option<BlurGpuPipeline>>,   // lazily built the first time ctx.gpu is Some
    pending: RefCell<Option<PendingBlurJob>>,          // an in-flight async dispatch, if any
    last_gpu_result: RefCell<Option<CompletedBlurJob>>, // most recent completed result + the fingerprint (radius, source pixel identity) it was computed from
}
```

`execute()`'s unmasked branch (the masked/bbox-consuming branch from Phase 3 is untouched, per RFC-001):

1. `ctx.gpu.is_none()` → CPU fallback (`Self::blur_pixels_static`), unconditionally. Always correct, always available.
2. `ctx.gpu.is_some()`:
   - If `last_gpu_result` matches this tick's actual fingerprint (radius + resolved source pixels) → use it.
   - Otherwise → CPU fallback **for this tick only** (never block waiting for GPU), and (re-)kick off a fresh async dispatch for the current fingerprint via `wasm_bindgen_futures::spawn_local`, writing into `pending`/`last_gpu_result` when it resolves.

**Target-conditional dispatch, not two designs:** `#[cfg(target_arch = "wasm32")]` uses the non-blocking `map_async` + `spawn_local` pipelined path described above. `#[cfg(not(target_arch = "wasm32"))]` (native, i.e. `cargo test`) may use a blocking read (`pollster::block_on` or `device.poll(Wait)`, exactly like the removed attempt) — native backends genuinely support blocking, so this isn't wrong there, only in the browser. This split is exactly the existing pattern `profiling.rs`'s `measure_ms` already uses (`#[cfg(target_arch = "wasm32")]` vs. not) — follow that precedent, don't invent a new one. This is also what makes the GPU path deterministically testable under `cargo test`, without mocking wgpu away.

**Required correctness detail — do not skip this:** `RenderExecutor`'s cross-tick cache (`compositor/executors/render.rs`) skips calling `execute()` again at all for a node whose parameters and resolved inputs haven't changed. A `BLUR` node with a genuinely static upstream graph would therefore only ever call `execute()` once — meaning if that single call fell back to CPU (no GPU result ready yet), it would **never get a chance to pick up the completed GPU result later**, staying stuck on CPU forever even after the async job finishes. Fix: `Blur::is_live()` must return `true` whenever `pending.borrow().is_some()` (there's an outstanding job worth checking on) — this forces re-execution on the next tick specifically so the operation gets a chance to consume a just-completed result and update its own cache, and naturally returns to `false`-driven caching once stable. `is_live()` already takes `&self`, so reading this from the same `RefCell` is a direct, no-new-mechanism fix.

### Numerical tolerance

GPU (WGSL) float math and CPU (Rust) float math for the same box-blur will not always produce bit-identical results (different summation order, different intermediate rounding). Tests comparing GPU output to CPU output must use a small tolerance (e.g. `(a - b).abs() < 1e-4` per channel), not exact equality — unlike every other consume-equivalence test in the bbox work (which are exact, since CPU-vs-CPU restricted/unrestricted comparisons have no such source of divergence).

## Phased work

**Phase 0 — `gpu` module foundation.** `GpuState`, generic shader/buffer helpers, `Context.gpu: Option<Arc<GpuState>>`, guarded non-blocking boot-time init. No operation uses it yet. Acceptance: app boots identically with or without WebGPU support in the browser (test both), `Context` still derives `Default`, full existing test suite unaffected.

**Phase 1 — `BLUR`'s GPU path.** The pipelined dispatch described above, target-conditional native/wasm32 split, `is_live()` fix, numerical-tolerance tests. Acceptance:
1. On a machine/browser with GPU support: after enough ticks for a job to resolve, `BLUR`'s output matches the CPU path within tolerance.
2. On a machine/browser without GPU support (or with `ctx.gpu` forced to `None` in a test): identical behavior to before this spec — CPU path, unconditionally.
3. A static-graph regression test proving the "stuck on CPU forever" bug above cannot happen — construct a graph, tick it enough times for a GPU job to complete without any input ever changing, assert the node's output eventually reflects the GPU result (or, if run without real GPU access in CI, assert `is_live()` returns `true` while `pending` is `Some`).
4. `cargo test` passes fully on native (real GPU dispatch exercised, blocking-OK path).

## Out of scope for this round

- **COMPUTE MODE user-facing switch** (CPU/GPU/AUTO selection UI). `Context.gpu: Option<...>` already *is* the AUTO behavior (GPU when available, CPU otherwise) with no user control needed. A manual override is real future work, not needed now — see the companion menu specification for where such a control would eventually register, if wanted.
- **Any operation besides `BLUR`.** The `gpu` module's helpers are deliberately generic (not blur-specific) so a second GPU-accelerated operation is a normal addition later — its own pipeline/shader/pending-state, reusing `GpuState` — not a redesign. Don't build a second one speculatively now.
- **Buffer-shrinking / the RAM-reduction backlog item.** Unrelated axis of work, already tracked separately in `PARKED_WORK.md`.

## Acceptance criteria (whole spec)

1. `cargo build`/`cargo test` clean on native.
2. App boots and is fully usable on a machine with no WebGPU support at all, with zero errors surfaced to the user.
3. On a WebGPU-capable machine, `BLUR` measurably uses the GPU path once warmed up (verifiable via the existing `Profile`/`ProfileEntry` timing instrumentation, or a debug counter — pick whichever is cheaper to wire up).
4. No blocking call reachable from the `wasm32` build target — audit every `wgpu` call in the new code and confirm no `PollType::Wait`/blocking `recv()` exists outside the `#[cfg(not(target_arch = "wasm32"))]` branch.
5. `is_live()`'s pending-job behavior is covered by the static-graph regression test above.
