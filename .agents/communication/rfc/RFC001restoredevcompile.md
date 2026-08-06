# RFC-001: Restore `dev` to a compiling, deployable state

**Status:** Urgent — blocks all other work, including the in-progress bounding-box Phase 3 rollout.
**Type:** RFC, not a Specification — this reverses an unreviewed direct-to-`dev` change, it doesn't design new behavior.

## Background

39 commits were pushed directly to `dev` (not through a branch/PR/evaluator, unlike every other change in this project) attempting a WebGPU-backed compute path for `BLUR`. `dev` currently fails to compile: `cargo build`/`cargo test` produce 47 errors, all cascading from one root cause — `Context` (`engine/src/compositor/context.rs`) gained a new field, `compute: Arc<dyn ComputeBackend>`, which has no meaningful default value, so `Context` lost its `#[derive(Default)]`. Nearly every test in the codebase constructs `Context` via `Context { meta: ..., ..Default::default() }` or `Context::default()`, including the entire bounding-box test suite (`BBOX_CONVENTIONS.md`) built over the last several PRs.

This landed *between* the `ghost` and `screen` Phase-3 bbox merges — meaning `screen`, `subtract`, and `hue_key`'s own evaluators built and reported against an already-broken tree.

This RFC is a **full removal**, not a patch-forward. The GPU/compute integration is being redesigned from scratch in a separate specification (WebGPU compute backend — see the companion document), because the current attempt has fundamental problems beyond the compile error (see "Why not patch it" below). Patching `Context`'s `Default` impl alone would restore compilation but leave broken, unreachable, and misleading code in the tree.

## Why not patch it forward

Documented here so nobody re-derives this later and wonders why a working-looking `wgpu` integration was deleted instead of fixed:

- **The GPU buffer readback blocks the thread.** `GpuBlur::read_buffer` (`engine/src/compute/gpu/blur.rs`) calls `slice.map_async(...)`, then `device.poll(PollType::Wait)`, then blocks on `mpsc::Receiver::recv()`. That only works on native wgpu backends (Vulkan/Metal/DX12 can block a thread waiting on the GPU). This app runs as WASM in a browser (per `CLAUDE.md`'s own first line), where WebGPU buffer mapping is *unavoidably* asynchronous — there is no blocking-wait available in that environment at all. This is not a bug to fix in place; the whole `fn blur(&self, ...) -> Vec<f32>` synchronous trait shape is wrong for the target platform.
- **GPU init is unconditional and unguarded at boot.** `ui/scripts/app.js`: `await wasmApp.init_gpu();` runs with no `try`/`catch`, before anything else in `boot()`. On any machine/browser without WebGPU support, or any GPU init failure, the promise rejects and the entire boot sequence — `applyOutputSize()`, `reportSelection()`, the `operationsLoaded` event, `startRenderLoop()` — never runs. The app fails to start at all. This is the opposite of the "GPU first, CPU fallback" behavior wanted.
- **`ComputeBackend` isn't an abstraction — it's one hardcoded method.** `trait ComputeBackend { fn blur(...); }`. No other operation has any way to use it.
- **It's disconnected even on its own terms.** `App.compute: Option<Arc<dyn ComputeBackend>>` defaults to `None`; `Context.compute` requires a non-optional value. Nothing resolves that gap for an ordinary render tick before `init_gpu()`/`set_compute_mode()` is called.
- **The COMPUTE MODE menu entry doesn't work either.** Its `MODE` parameter is declared (`ParameterKind::Enum(["CPU","GPU","AUTO"])`) but `ui/scripts/engine/menu.js`'s `renderOperationButtons()` never reads or renders `op.parameters` — only real graph nodes get parameter steppers, through a completely different code path (`node_inputs`/`nodeEditContexts.js`) a `SystemMenuDescriptor` (no node id) can't plug into. Clicking it has nothing to actually select a mode with.
- **`engine/src/gpu/pipeline.rs`'s `BlurPipeline` is dead code** — byte-for-byte the same pipeline-construction logic as `compute/gpu/blur.rs`'s `GpuBlur::new()`, never actually constructed or used anywhere.

None of this is fixable by restoring `Default` on `Context` alone. The companion WebGPU specification designs the real thing; this RFC just gets the tree back to a known-good, fully-working state first.

## Removal list (exact)

**Delete entirely:**
- `engine/src/compute/` (`mod.rs`, `backend.rs`, `params.rs`, `cpu/mod.rs`, `gpu/mod.rs`, `gpu/blur.rs`)
- `engine/src/gpu/` (`mod.rs`, `context.rs`, `pipeline.rs`, `shaders/blur.wgsl`)

**Edit:**
- `engine/src/compositor/context.rs`: remove the `compute: Arc<dyn ComputeBackend>` field and its `use crate::compute::backend::ComputeBackend;` import; remove the `ComputeMode` enum entirely (its only consumers — `App.compute_mode`, `compute::create_backend`, `set_compute_mode` — are all being removed too); restore `#[derive(Clone, Default)]` on `Context`.
- `engine/src/compositor/mod.rs`: remove `ComputeMode` from the `pub use context::{ Context, ComputeMode, Meta };` re-export line.
- `engine/src/operations/transform/blur.rs`: in `execute()`'s unmasked (`else`) branch, replace `ctx.compute.blur(&source.pixels, source.width, source.height, self.radius_px)` with a direct call to the existing `Self::blur_pixels_static(&source.pixels, source.width, source.height, self.radius_px)` — the same function `CpuBackend` was only ever delegating to. **The masked branch (the Phase 3 bbox-consuming path, `compute_within_bbox`/`natural_bbox`) is untouched — do not modify it.**
- `engine/src/app.rs`: remove the `compute: Option<Arc<dyn ComputeBackend>>` field, `compute_mode: ComputeMode` field, `init_gpu()` method, `set_compute_mode()` method, and their now-unused imports (`ComputeBackend`, `ComputeMode`; keep `Arc` if `app.rs` still uses it elsewhere, which it does for other fields — check before removing the import).
- `engine/src/compositor/system.rs`: remove only the `inventory::submit! { ... }` block registering `compute_mode`. **Leave `SystemMenuDescriptor`, `SystemMenu`, and `descriptors()` in place** — they'll compile fine with zero registered entries (an empty `Vec`), and the companion menu specification is where that framework's own fate gets decided; don't preempt it here.
- `engine/Cargo.toml`: remove the `wgpu`, `pollster`, and `bytemuck` dependencies — all three are exclusively used by the code above.
- `ui/scripts/app.js`: remove the `await wasmApp.init_gpu();` line from `boot()`.

**Do not touch:** `ui/scripts/core/wasm.js`'s `systemMenus`/`get_system_menus()` wiring, or `menu.js`'s `systemMenus` handling — leave both in place (they'll just carry an empty array once `compute_mode` is gone; harmless, and again, the menu specification's territory, not this RFC's).

## Acceptance criteria

1. `cargo build` and `cargo test` succeed with zero errors on `engine/`.
2. The full pre-existing test suite passes (re-run and report the exact count — it was 254 passing as of the `BLUR` Phase 3 merge, before `chromakey`/`ghost`/`screen`/`subtract`/`hue_key` added their own tests, so the real number is higher now).
3. `BLUR`'s masked-path tests (`consume_equivalence_*`, `a_smaller_mask_bbox_computes_strictly_fewer_pixels_*`, `checkerboard_resize_move_geometric_mask_end_to_end_*`) all still pass, unmodified, proving the Phase 3 bbox work was not disturbed by this cleanup.
4. `grep -rE "wgpu|ComputeBackend|GpuBackend|GpuContext|ComputeMode|compute_mode|init_gpu" engine/src ui/scripts` returns **zero matches**.
5. `engine/Cargo.toml` no longer lists `wgpu`, `pollster`, or `bytemuck`.
6. The app boots in a browser without calling `init_gpu()`, and the `PROJECT` menu category no longer appears at all (nothing is registered under it — this should require no menu-hiding code, since `renderCategoryButtons()` already only shows categories with real registered entries).
7. No file outside the removal/edit list above is touched.
