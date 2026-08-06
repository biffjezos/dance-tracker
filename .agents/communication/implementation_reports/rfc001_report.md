# RFC-001 Implementation Report: Restore `dev` to a compiling, deployable state

Branch: `claude/rfc-001-restore-dev-compile` (based on real `origin/dev` tip `38f94dd`)

## Summary

Removed the unreviewed WebGPU compute-path attempt (`engine/src/compute/`, `engine/src/gpu/`) exactly per RFC-001's removal list, and made the corresponding edits to `context.rs`, `compositor/mod.rs`, `blur.rs`, `app.rs`, `system.rs`, `Cargo.toml`, and `app.js`.

## Changes (matches the RFC's removal/edit list)

- **Deleted:** `engine/src/compute/` (all 5 files) and `engine/src/gpu/` (all 4 files).
- **`compositor/context.rs`:** removed the `compute: Arc<dyn ComputeBackend>` field and its import, removed the `ComputeMode` enum, restored `#[derive(Clone, Default)]` on `Context`.
- **`compositor/mod.rs`:** dropped `ComputeMode` from the `pub use context::{...}` line.
- **`operations/transform/blur.rs`:** unmasked branch now calls `Self::blur_pixels_static(...)` directly instead of `ctx.compute.blur(...)`. Masked (bbox-consuming) branch untouched.
- **`app.rs`:** removed `compute`/`compute_mode` fields, `init_gpu()`, `set_compute_mode()`, and the now-unused `ComputeMode`/`ComputeBackend` imports. Kept the `Arc` import (still used for `U8Image`/pixel-source construction elsewhere in the file).
- **`compositor/system.rs`:** removed only the `compute_mode` `inventory::submit!` block. `SystemMenuDescriptor`/`SystemMenu`/`descriptors()` left in place. `ParameterKind` import dropped since it became unused once the block was gone.
- **`Cargo.toml`:** removed `wgpu`, `pollster`, `bytemuck`.
- **`app.js`:** removed the `await wasmApp.init_gpu();` line from `boot()`.

## One deviation from the literal removal list — flagging for confirmation

`Cargo.toml` also had `wasm-bindgen-futures = "0.4.76"`, added in the same 39-commit spree (`bcf02be "wasm-bingen-futures"`, landing right after the webgpu-skeleton commits) but not named in RFC-001's removal list. I confirmed it has zero usages anywhere in `engine/src` (`grep -rn wasm_bindgen_futures` — no matches) — it's dead weight left over from the same removed initiative, presumably intended to support `init_gpu()`'s async signature but never actually referenced. I removed it because, like `wgpu`, it requires fetching from `static.crates.io`, which is policy-blocked in this sandbox — leaving it in would have made `cargo build` fail here for a reason unrelated to any of the code this RFC is actually about. If it's wanted back (e.g. reserved for the companion WebGPU spec), it's a one-line re-add; happy to revert this part specifically.

## Verification

- `cargo build` — clean, zero errors, zero warnings other than 3 pre-existing unrelated ones (`inventory.rs` doc comment, two `never read` fields in `graphics/geometry.rs`, both present before this change).
- `cargo test` — **283 passed, 0 failed** (up from the 254 baseline cited in the RFC, which predates `chromakey`/`ghost`/`screen`/`subtract`/`hue_key`'s own tests).
- `cargo test --lib blur::` — all 18 tests pass unmodified, including the three named masked-path tests (`consume_equivalence_*`, `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one`, `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off`) — confirms Phase 3 bbox work was undisturbed.
- `grep -rE "wgpu|ComputeBackend|GpuBackend|GpuContext|ComputeMode|compute_mode|init_gpu" engine/src ui/scripts` — zero matches.
- `engine/Cargo.toml` no longer lists `wgpu`, `pollster`, or `bytemuck` (nor, per the deviation above, `wasm-bindgen-futures`).
- `git diff origin/dev --stat` — touches exactly the files in the RFC's removal/edit list, plus `Cargo.toml`'s one extra line covered above. No other file touched.
- Did not verify item 6 (app boots in a real browser, `PROJECT` menu category disappears) — no browser test environment available in this session; this is a static-code claim only, consistent with `system.rs`'s `compute_mode` being the sole entry ever registered under `PROJECT`/`SETTINGS` and `renderCategoryButtons()`'s existing empty-category behavior, but worth a manual check before merge.

## Build-environment note

No workaround branch was needed this time — this RFC's own changes are exactly what makes `cargo build` work again in this sandbox (once the also-unused `wasm-bindgen-futures` dependency is dropped too, per the deviation above). This is unlike the SCREEN/SUBTRACT/HUE KEY rounds, which needed the pre-breakage-base verification workaround precisely because this RFC hadn't landed yet.
