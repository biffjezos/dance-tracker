# Evaluation: WebGPU operations Phase 2 — resampling ops

**Branch:** `claude/agent-setup-prep-oyj2ri` (not yet merged)
**Spec:** `SPECwebgpuoperations.md` (Phase 2, the final phase), pattern from `SPECwebgpucomputebackend-1.md`, shape approved in Phases 0/1.1/1.2/1.3
**Report:** `.agents/communication/implementation_reports/webgpu/operations_Phase2_resampling_report.md`
**Commits:** `0f7d05b` (`resize.rs`), `0634442` (`move_op.rs`) — two independent, one file each. **This closes out `SPECwebgpuoperations.md` in its entirety.**

## 0. Build-environment — same restriction, re-confirmed

`index.crates.io`/`static.crates.io` still 403 in this session. Manual/static review only; **build/test acceptance criteria remain UNVERIFIED**, per `ENVIRONMENT_DIAGNOSTICS.md`, same as every prior phase.

## 1. `RESIZE` — ✅ correct, plus a full analysis of RFI Q2 below

- No `MASK` input, confirmed via `metadata()`'s own comment (dimension mismatch at any scale ≠ 100% makes a mask invalid here) — GPU dispatch unconditional when available. ✅
- Defensively gated on `algorithm == ResizeAlgorithm::NearestNeighbor`, the only variant that exists and the only one the shader implements — same defensive pattern as `RGB_TO_HSV`'s `format == Hsv` gate. ✅
- `inv_x`/`inv_y` (`100.0 / scale`) computed Rust-side in `f64` — exactly `resize_pixels`'s own formula — then narrowed to `f32` for upload, not recomputed from raw scale inside the shader. Traced this against `resize_pixels` directly: same formula, same order of operations. ✅
- Coordinate math (`cx = width/2`, `src_x = cx + (id.x + 0.5 - cx) * inv_x`) matches `resize_pixels` exactly, modulo the `f32`/`f64` precision gap discussed in §3. ✅
- Out-of-frame handling: shader writes `(0,0,0,0)` and returns early; CPU's `continue` leaves the pixel at its `vec![T::default(); ...]` initial value, which is also `0`/transparent for both `u8` and `f32` outputs. Same effective default. ✅
- `sx = u32(src_x)` truncates toward zero in WGSL, matching Rust's `src_x as u32` truncation — both only ever operate on already-non-negative `src_x` (filtered by the preceding bounds check), so no divergent rounding-direction risk between the two truncation operators themselves. ✅
- Fingerprint (`source`, `scale_x_bits`, `scale_y_bits`) captures everything `resize_pixels` depends on. ✅

## 2. `MOVE` — ✅ correct, masked-path scope decision consistent with `MULTIPLY`'s already-confirmed reasoning

- Confirmed against `dev`'s pre-existing `move_op.rs`: `execute()`'s masked path never called `find_bbox`/`compute_within_bbox` — only `output_bbox()` does — so it genuinely is, and remains, unrestricted full-frame `move_pixels` + `apply_mask`, exactly as the report claims. This diff leaves that path completely untouched and only attempts GPU dispatch when `mask.is_none()`. ✅ Same conclusion as the Phase 1.3 evaluation reached for `MULTIPLY`, and for the identical two reasons (the spec's blanket rule has no "unless already full-frame" exception; keeping bbox-migration and GPU-acceleration as separable workstreams). Nothing about `MOVE` specifically changes that calculus — same shape of operation, same shape of gap.
- Coordinate math (`src_x = id.x - offset_x`, no center-relative term) matches `move_pixels`'s simpler offset-only formula exactly — correctly omits `RESIZE`'s `cx`/scale term, as it should. ✅
- **Struct-literal fix claim verified directly, not just trusted:** the report claims `Move { offset_x, offset_y }` was constructed as a bare literal six times in `move_op.rs`'s own test module (not just via `Move::new()`), all six needing `..Move::new()` added. Counted them directly in the diff — exactly six occurrences, all updated, no asserted test values changed. `git grep "Move {"` across the branch turns up zero remaining unfixed literals. ✅
- Fingerprint (`source`, `offset_x_bits`, `offset_y_bits`) captures everything `move_pixels` depends on. ✅

## 3. RFI Q1 — `MOVE`'s masked-path scope, does the `MULTIPLY` reasoning still hold?

Yes, unchanged. Re-derived the two reasons independently rather than just citing the prior answer: (a) the spec's blanket rule ("GPU dispatch only ever replaces the unmasked path") applies uniformly across every phase in the document, with no carve-out for an operation whose masked path happens to already be full-frame; (b) `MOVE`, like `MULTIPLY`, will eventually need its own bbox migration as separate future work, and that migration should start from a plain CPU masked path (the same starting point every already-migrated operation's own migration had), not one that's already GPU-accelerated and now needs bbox-restriction logic retrofitted onto a GPU dispatch. There's nothing operation-specific about `MOVE` that changes either point — it's the same shape of gap as `MULTIPLY`'s.

## 4. RFI Q2 — `RESIZE`'s `f32`/`f64` boundary comparison: a real, structurally-guaranteed-possible divergence, correctly flagged, non-blocking

This deserves more than "same precision-tolerance story as before," and I want to be precise about why it both *is* a real, distinct risk and *isn't* something to block on.

**Why it's real, not just theoretical:** `src_x`'s continuous mathematical value crossing exactly `width` (or `0`) is a measure-zero event in general, but the codebase's own scale/offset parameters are user- and animation-controlled floats, not restricted to "safe" values — an animated `SCALE_X` sweeping continuously through a range will, at some exact frame, pass arbitrarily close to a boundary-crossing value for *some* pixel column, at which point `f32`'s coarser precision (~7 decimal digits vs. `f64`'s ~15-16) can genuinely round to the opposite side of the `>= width` comparison than `f64` does. This is categorically different from `BLUR`'s/`ADD`'s/etc. numerical-tolerance story, where a small `f32` vs. `f64` difference stays a small difference in a continuous output — here it can flip a discrete branch (sampled pixel vs. transparent), the same shape of risk as `CHROMAKEY`'s and `RING`'s threshold comparisons (flagged in the Phase 1.1/1.2 evaluations), but landing specifically at the frame edge rather than an arbitrary color/distance threshold — a place a viewer's eye is more likely to catch a flickering edge pixel during an animated resize/move than a flickering interior chroma-key pixel.

**Why it's still non-blocking, not something to request a fix for:**

1. **It can't be fixed by writing different code, only by not using `f32` at all.** WGSL's `compute` stage (the shading language this entire spec's GPU path is built on) has no native `f64` arithmetic — `f32` is the working precision throughout `wgpu`'s standard compute pipeline. There is no alternative shader formulation that closes this gap while still running on GPU; the only way to eliminate it entirely is to not GPU-accelerate `RESIZE` at all, which isn't a reasonable ask for a phase whose entire purpose is doing exactly that.
2. **The failure mode is self-limited and non-accumulating.** A boundary-straddling pixel affects, at most, that one pixel on that one tick, for that one exact parameter combination — it doesn't compound across frames or corrupt anything beyond a single edge pixel's alpha for as long as the parameters sit exactly on the boundary (an instant, for a continuously-animated value).
3. **Same category already accepted twice this spec**, for the same underlying reason (mixed-precision thresholding is inherent, not a coding defect) — `CHROMAKEY`'s distance threshold and `RING`'s ring-boundary hit test both got the identical "informational, not blocking" treatment.

**What I did check, beyond accepting the report's own "didn't find a concrete failing case":** worked through the algebra of when the exact boundary condition is reachable — solving `cx + (x + 0.5 - cx) * inv_x = width` for `x` shows it requires a very specific relationship between `x`, `width`, and `scale_x` (not a generic occurrence for ordinary parameter choices, consistent with the report's own finding that their test data didn't hit it), which is exactly why this is a narrow, rare-in-practice risk rather than a systematic one that would show up as generally-wrong output for typical resize/move operations.

**One suggestion, not a requirement:** if this ever needs harder confidence later (e.g., if a user-reported flicker traces back to this), a targeted regression test using parameters algebraically chosen to land exactly on a boundary (the same "construct the exact adversarial case directly" approach `RFC-003`'s fix used, rather than relying on random test data to stumble onto it) would let a future evaluator confirm the behavior is at least *consistent* between GPU and CPU paths even at the boundary, even though it can't make them agree in the cases where `f32`/`f64` genuinely round differently. Not requesting this now — flagging it as a good idea if the risk ever needs to move from "theoretical, bounded" to "actively characterized."

## 5. Recommendation

**✅ Approve both `RESIZE` and `MOVE`.** No blocking or major defects in either. No RFC needed. This closes out `SPECwebgpuoperations.md` in its entirety — Phases 0, 1.1, 1.2, 1.3, and 2 are all now implemented and reviewed.
