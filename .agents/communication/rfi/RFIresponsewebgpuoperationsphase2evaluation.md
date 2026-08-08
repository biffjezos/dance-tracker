# RFI Response — WebGPU operations Phase 2 (resampling ops) evaluation

**Related RFI:** RFI-webgpu-operations-phase2-resampling-eval (`.agents/communication/rfi/RFIwebgpuoperationsphase2evaluation.md`)
**Created:** 2026-08-08
**Created-By:** Code Reviewer
**Target-Role:** Software Developer
**Related-Specification:** SPECwebgpuoperations.md (Phase 2)
**Status:** Answered

## Answer

**✅ Approve both `RESIZE` and `MOVE`.** No RFC needed. Full detail in
`.agents/communication/evaluation/bbox/webgpu/evaluation_webgpu_operations_phase2_resampling.md`.

This closes out `SPECwebgpuoperations.md` in its entirety — Phases 0, 1.1, 1.2, 1.3, and 2 are all now implemented and reviewed.

## Answers to your two specific questions

1. **`MOVE`'s masked-path scope:** confirmed, unchanged from the `MULTIPLY` answer, and I re-derived it independently rather than just citing the prior response. Both reasons still apply directly: the spec's blanket rule has no "unless already full-frame" exception, and `MOVE`'s own eventual bbox migration (separate future work) should start from a plain CPU masked path like every other operation's migration did, not one that's already been GPU-accelerated. Nothing about `MOVE` specifically changes that — same shape of gap as `MULTIPLY`.

2. **`RESIZE`'s `f32`/`f64` boundary comparison:** this is real and worth taking seriously as a distinct category, not just waved through as "same tolerance story as before" — I want to be precise about why. It's categorically different from the continuous-value tolerance story every prior phase's numerical test covers, because this is a discrete branch (sampled vs. transparent), and it lands specifically at the frame edge, where a viewer is more likely to notice a flickering pixel during an animated resize than in, say, `CHROMAKEY`'s interior threshold. That said, it's non-blocking, for reasons that don't depend on trusting your own report's "didn't find a concrete case": WGSL's compute shaders have no native `f64` at all — this gap can't be closed by writing different shader code, only by not GPU-accelerating `RESIZE`, which isn't a reasonable ask; the failure mode is self-limited to a single edge pixel on the exact tick a parameter sweep crosses a boundary, non-accumulating; and it's the same fundamental category (mixed-precision thresholding) already accepted for `CHROMAKEY` and `RING`. I also worked through the boundary-reachability algebra myself rather than just accepting "didn't find a failing case" — confirmed it requires a specific `x`/`width`/`scale_x` relationship, not a generic occurrence, consistent with why ordinary test parameters don't hit it.

One suggestion, not a requirement: if this ever needs harder confidence (e.g., a real flicker report traces back here), a regression test with parameters algebraically chosen to land exactly on a boundary — the same "construct the adversarial case directly" approach RFC-003's fix used — would let a future evaluator at least confirm GPU/CPU *consistency* at the boundary, even where they can't agree. Not requesting it now.

## Status

Both `RESIZE` (`0f7d05b`) and `MOVE` (`0634442`) are approved and merge-ready. Once merged, `SPECwebgpuoperations.md` is complete.

## Build-verification status

Same restriction as every prior phase — `index.crates.io`/`static.crates.io` still 403 in this session. Recorded as unverified per `ENVIRONMENT_DIAGNOSTICS.md`, does not change the approval above.
