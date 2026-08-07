# RFI Response — WebGPU compute backend spec

**In response to:** RFI Packet, WebGPU compute backend spec (2026-08-06)
**Responding Role:** Software Architect
**Status:** Answered — `SPEC-webgpu-compute-backend.md` updated in place with all four clarifications, so the implementer works from one self-sufficient document rather than cross-referencing this response.

First, a status note relevant to context: I independently verified RFC-001 and RFC-002 are merged and correct before answering these — pulled `dev` fresh (`0b070ff`), ran `cargo test --lib` myself: clean build, 287 passed, 0 failed. The tree these RFIs are being asked against is in the state the spec assumes.

---

## RFI-001 — Fingerprint mechanism

**Answer: pointer identity via the existing `value_ptr_eq` helper (`compositor/value.rs`), not a hash — but not `Arc::ptr_eq` on the post-`FloatImage::from_value` clone either.**

`FloatImage::from_value` (`graphics/float_image.rs`) *clones* the `FloatImage` out of its `Arc` on the `Value::FloatImage` branch (`Ok((**float_image).clone())`) — so by the time `Blur::execute()` has called it, the resulting local `FloatImage` has no meaningful pointer identity back to the originally-wired value. Comparing pointers *after* that clone would never match, defeating the mechanism entirely.

The fix: capture the fingerprint from the **wired `Value` itself**, before `FloatImage::from_value` runs — `find_input(inputs, Input::Source)` already hands you the `&Value`; clone it (cheap — `Value::clone()` on `Frame`/`Mask`/`Image`/`FloatImage`/`Video` is an `Arc::clone`) into `PendingBlurJob`/`CompletedBlurJob`'s stored fingerprint, and compare on the next tick via `crate::compositor::value::value_ptr_eq(&new_value, &old_value)` — the exact function `RenderExecutor`'s own cross-tick cache already uses for this exact purpose. Reusing it here means BLUR's GPU fingerprint is provably as safe as the rest of the engine's caching (if pointer-identity reuse were a real risk, `RenderExecutor`'s cache would already be unsound, and it isn't).

Fingerprint = `(radius_px: u32, source_value: Value)`, compared as `new_radius == old_radius && value_ptr_eq(&new_source_value, &old_source_value)`.

## RFI-002 — `wgpu` version / wasm32 feature flags

**Answer, evidence-based, not asserted from memory:** the removed attempt's `Cargo.toml` had `wgpu = "30.0.0"`, `pollster = "1.0.1"`, `bytemuck = "1.25.2"` alongside the *currently still-pinned* `wasm-bindgen = "0.2.126"` and `web-sys = "0.3.103"`. I confirmed directly (I ran the build myself before RFC-001 landed) that this combination does **not** produce a dependency-resolution conflict — `cargo build` progressed past resolution and failed on ordinary Rust type errors in our own code (`E0277`/`E0599` on `Context::default()`), which could only happen after successful crate resolution. Had the versions conflicted, that would have surfaced first, before any of our own code got type-checked.

So: re-add exactly `wgpu = "30.0.0"`, `pollster = "1.0.1"`, `bytemuck = "1.25.2"` as the starting point — confirmed compatible with what's already pinned, though not verified as the *latest* or most feature-complete versions available. **`wasm-bindgen-futures = "0.4.76"` is already a dependency** (pre-dates this work, used elsewhere) — don't add a second pin for it. After re-adding, run `cargo tree -p wgpu` and pin whatever the lockfile actually resolves rather than leaving loose version ranges, so a future `cargo update` can't silently drift into an unverified combination.

## RFI-003 — Ownership of `BlurGpuPipeline`/`PendingBlurJob`/`CompletedBlurJob`

**Answer: `operations/transform/blur.rs`. Confirmed, not implied.** The spec's design principle is explicit that the shared `gpu` module stays generic (device/queue/shader/buffer helpers only, no operation-specific method) — these three types are BLUR-specific by definition, so they belong where `Blur`'s own struct fields do. The spec text has been updated to state the module path directly rather than leave it inferred.

## RFI-004 — App → Context handoff

**Answer, against the actual current code, not a general description:** there is exactly one wiring seam — `App::context(&self, preview: bool) -> Context` (`engine/src/app.rs`, private method, ~line 517), called by both `render_tick` and `preview_tick`. It already builds `meta`, `resources`, and `input_bboxes` fresh every call; add `gpu: self.gpu.clone()` there, alongside them.

One thing worth flagging from directly reading the *removed* code while confirming this: the prior attempt's version of this same function had `compute: self.compute.clone().expect("Compute backend not initialized")` — an `.expect()` that would have **panicked on every single render tick** until `init_gpu()`/`set_compute_mode()` resolved, which given RFI's own boot-sequencing concerns could easily be never, or race with the first tick. The new `App.gpu` field must be `Option<Arc<GpuState>>` (matching `Context.gpu`) and read via a plain `.clone()`, never `.expect()`/`.unwrap()` — `None` is a valid, expected, common state (GPU not yet ready, or genuinely unavailable), not an error condition.

Phase 0's acceptance criteria in the spec now include an explicit check for this seam specifically (not just "`Context` still derives `Default`"): after `init_gpu()` resolves in a test/integration context, the *next* `App::context()` call must return `Context.gpu = Some(...)` — proving the value actually becomes reachable from a live render tick, not just that the field exists.

---

All four answers are now incorporated directly into `SPEC-webgpu-compute-backend.md` (attached, replaces the previous version) — the implementer should work from that file alone.
