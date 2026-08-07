# Implementation Report: WebGPU compute backend, Phase 0 — `gpu` module foundation

Branch: `claude/webgpu-phase0-foundation` (based on real `origin/dev` tip `0b070ff`, post-RFC-002)
Spec: `SPECwebgpucomputebackend2.md`

## ⚠️ Could not be built or tested in this sandbox — read this first

Unlike every prior round in this project, **this one has no working-around it.** `static.crates.io` is policy-blocked in this sandbox (confirmed via the proxy status endpoint, same as the RFC-001 investigation), and none of `wgpu`'s dependency tree is cached locally. RFC-001 could dodge an equivalent problem by verifying on a commit that predated the broken dependency and patching a scoped diff onto the real tip — that doesn't work here, because **`wgpu` being present and compiling is the actual deliverable of this spec.** There is no clean base without it to fall back to.

Concretely: the moment `wgpu`/`pollster`/`bytemuck`/`wasm-bindgen-futures` are added back to `Cargo.toml`, `cargo build` fails immediately trying to fetch the dependency tree:

```
error: failed to download from `https://static.crates.io/crates/allocator-api2/0.2.21/download`
[56] Failure when receiving data from the peer (CONNECT tunnel failed, response 403)
```

This happens before a single line of my own code is even compiled — it's a pure network/registry problem, not a code problem. **I have not run `cargo build` or `cargo test` successfully against this branch at all.** Per your explicit instruction, I'm proceeding with the implementation anyway, documented accordingly, pending a resolution from the technical advisor.

One thing I *could* verify: `cargo build` resolves a full `Cargo.lock` before it tries to download anything, and that resolution succeeded — `wgpu 30.0.0`, `pollster 1.0.1`, `bytemuck 1.25.2`, `wasm-bindgen-futures 0.4.76` all resolved exactly as the spec's "confirmed dependency versions" section specifies, with no version conflicts against the existing `wasm-bindgen 0.2.126`/`web-sys 0.3.103` pins. That `Cargo.lock` is committed. I could not run `cargo tree -p wgpu` (it needs the actual crate sources, not just the lock file, so it hit the same network block) — the committed lockfile is the closest available substitute for "pin whatever actually resolved."

**Everything below this point is unverified code**, written as carefully and consistently with the spec (and with wgpu API shapes I have direct evidence for — see "Where I made judgment calls," below) as I can manage without a compiler. It needs a real build before merge.

## Summary of the change

Implements Phase 0 exactly as scoped: the `gpu` module foundation, `Context`/`App` wiring, and guarded non-blocking boot-time init. No operation uses any of this yet — that's `SPEC-webgpu-operations.md`'s job, which this phase unblocks.

### `engine/src/gpu/mod.rs` (new)

- `GpuState { device, queue }`, built via `GpuState::new().await -> Result<Self, String>` — the one place `wgpu::Instance`/`request_adapter`/`request_device` are called. Resolves to `Err`, never panics, on a machine with no compatible adapter.
- Generic, operation-agnostic helpers: `create_shader`, `create_compute_pipeline` (takes a shader + bind group layouts + entry point — no operation-specific method anywhere in this file), `upload`, `create_buffer`, `create_bind_group`, `dispatch`, `copy_buffer_to_buffer`.
- Readback splits by target, per the spec's "target-conditional dispatch, not two designs" guidance, mirroring `profiling::measure_ms`'s existing `#[cfg(target_arch = "wasm32")]` split:
  - `read_buffer_blocking` (native only, `#[cfg(not(target_arch = "wasm32"))]`): `map_async` + `device.poll(PollType::Wait)` + a blocking `mpsc::Receiver::recv()`. Native backends genuinely support this.
  - `read_buffer_async` (wasm32 only): never calls `device.poll()` and never blocks. Resolves via a small hand-rolled `Future` (`MapReadyState`/`MapReadyFuture`, `Rc<RefCell<...>>`-backed since wasm32 is single-threaded) that's woken directly from `map_async`'s own callback. No extra async-utility crate needed for this one oneshot signal.

### `engine/src/compositor/context.rs`

Added `pub gpu: Option<Arc<GpuState>>`. `Context` still derives `#[derive(Clone, Default)]` unchanged — `Option<T>::default()` is `None` regardless of whether `T` implements `Default`, so `GpuState` (which has no `Default` impl, nor should it) doesn't break this. Every existing `..Default::default()`/`Context::default()` test-construction site across the codebase is untouched.

### `engine/src/app.rs`

- New `gpu: Option<Arc<GpuState>>` field on `App`, `None` at construction.
- New `pub async fn init_gpu(&mut self) -> Result<(), JsValue>` — calls `GpuState::new()`, sets `self.gpu = Some(Arc::new(gpu))` on success.
- `context()` (the exact seam named in the spec, ~line 517) now sets `gpu: self.gpu.clone()` — a plain clone, never `.expect()`'d. This is the fix for the removed attempt's most severe wiring bug (`self.compute.clone().expect("Compute backend not initialized")`, which panicked on every tick until GPU init resolved).

### `ui/scripts/app.js`

`boot()` now calls `wasmApp.init_gpu().then(...).catch(...)` as a bare fire-and-forget promise, immediately after `wasmApp` is constructed — never `await`ed inline, so a rejected promise (no WebGPU support, adapter denied, etc.) cannot block or break the rest of `boot()`. I used the JS-side `.then()/.catch()` option the spec explicitly permits as an alternative to `wasm_bindgen_futures::spawn_local`, since `init_gpu` is already exposed as a `#[wasm_bindgen]` async method on `&mut self` and awaiting it from JS-side `.then()` keeps the borrow entirely within wasm-bindgen's own generated Promise glue — no need to invent a `Rc<RefCell<App>>`-style self-mutation-across-`spawn_local` pattern, which would have been real new complexity for no behavioral difference.

### `engine/Cargo.toml`

Re-added `wgpu = "30.0.0"`, `pollster = "1.0.1"`, `bytemuck = "1.25.2"`, `wasm-bindgen-futures = "0.4.76"` — the last one had been dropped in RFC-001 as dead code (correctly, at the time); it's genuinely needed again now for `spawn_local`/async glue... except this round I ended up not needing `spawn_local` at all (see `app.js` above) — I left the dependency in per the spec's explicit instruction to reuse it (`SPEC-webgpu-operations.md`'s own per-operation dispatch work will need it for its `spawn_local` calls, even though Phase 0 itself doesn't call it directly).

## Where I made judgment calls (please double check these against a real build)

I pulled the removed attempt's deleted `gpu/context.rs`/`gpu/pipeline.rs`/`compute/gpu/blur.rs` (via `git show f880fe5^:...`) as a reference for wgpu 30's API shape, since RFC-001's own evaluation confirmed that code got *past* dependency resolution and only failed on ordinary Rust type errors in our own code (`E0277`/`E0599`) — meaning the overall shapes (single-argument `request_device`, `request_adapter` returning a `Result` not an `Option`, `PipelineLayoutDescriptor.bind_group_layouts` apparently expecting `&[Option<&BindGroupLayout>]` rather than `&[&BindGroupLayout]`, `PipelineLayoutDescriptor.immediate_size` instead of `push_constant_ranges`) are probably right, or at least were far enough along to survive resolution. I kept those shapes.

One place I deliberately diverged from that reference: its `read_buffer` called `slice.get_mapped_range().expect("Failed to map GPU buffer")` — `.expect()` on a method that, in every wgpu version I have firsthand knowledge of, returns a `BufferView` directly rather than a `Result`. Since the RFC-001 audit specifically named `E0599` ("no method found") among the removed code's errors, and `.expect()` on a non-`Result`/`Option` type is exactly the shape of an `E0599`, I judged this was more likely to be one of those latent errors than a real part of the wgpu 30 API, and wrote `read_buffer_blocking`/`read_buffer_async` without it (`let data = slice.get_mapped_range();`, no `.expect()`). If a real build disagrees, this is a one-line fix in `gpu/mod.rs`.

## Tests added (native, `#[cfg(test)]` in `gpu/mod.rs`)

1. `gpu_state_new_resolves_to_a_result_without_panicking` — calls `GpuState::new()` via `pollster::block_on`, asserts it returns a `Result` either way (doesn't require an adapter to actually be present) rather than panicking. This is the concrete guarantee `Context.gpu = None`'s whole "expected, not an error" design rests on.
2. `a_trivial_pipeline_round_trips_through_upload_dispatch_copy_and_readback` — if a real GPU adapter is available, builds a trivial "multiply by 2" WGSL compute shader, exercises the full `create_shader` → `create_compute_pipeline` → `upload`/`create_buffer` → `create_bind_group` → `dispatch` → `copy_buffer_to_buffer` → `read_buffer_blocking` pipeline end to end, and asserts the output matches within `1e-4` (per the spec's numerical-tolerance guidance). Skips gracefully (prints and returns) if no adapter is available, rather than failing — consistent with how `GpuState::new()` itself treats "no GPU" as an ordinary, ungrave outcome.

**Neither test has actually been run.** Even setting aside the network block, I don't know whether this sandbox's container has any GPU adapter (hardware or software/Lavapipe-style) available to a native wgpu backend at all — test 2 is written to degrade gracefully either way, but that graceful-degradation path itself is unverified too.

## What I could not verify or attempt at all

- **AC1 (`cargo build`/`cargo test` clean on native):** not run — see the blocker at the top.
- **AC2 (app boots with/without WebGPU support):** not run — no browser test environment available in this session (same limitation flagged in RFC-001's report), and the code couldn't even compile here to begin with.
- **AC3 (`Context`/`App` carry a working, correctly-wired `gpu: Option<Arc<GpuState>>`):** wired by direct code inspection (see "`context()`" above) but not executed.
- **The spec's own named Phase 0 acceptance test** ("after `init_gpu()` resolves in a test/integration context, the *next* `App::context()` call returns `Context.gpu = Some(...)`"): `App` is `#[cfg(target_arch = "wasm32")]`-gated entirely, so it has never had any native unit tests in this codebase (I checked — zero `#[test]`s exist in `app.rs` today), and I have no wasm32/browser test runner available in this sandbox. I did not add a `wasm-bindgen-test`-based test for this, since I'd have no way to verify it even compiles, and an unexecuted, unverified test file risked looking like fabricated coverage. The wiring itself (`self.gpu.clone()` inside `context()`, set from `init_gpu()`'s own `self.gpu = Some(...)`) is a two-line, directly-inspectable guarantee — I'm relying on code review to confirm it rather than a test I can't run, the same category of gap RFC-001's evaluator accepted for its own browser-only AC6.
- **AC4 (no blocking call reachable from wasm32):** audited by direct inspection — `device.poll(PollType::Wait)` and the blocking `.recv()` both appear exactly once, both inside `read_buffer_blocking`, which is the only thing in the file gated `#[cfg(not(target_arch = "wasm32"))]`. `grep`'d directly to confirm placement. Not compiler-verified that the `#[cfg]` attribute itself is syntactically correct, though it matches `profiling.rs`'s own established pattern exactly.

## Next steps

This needs a real `cargo build`/`cargo test` pass in an environment with `static.crates.io` access before it's mergeable — I'd treat everything above as a first draft pending that, not a final review-ready state. Happy to iterate immediately once that's available.
