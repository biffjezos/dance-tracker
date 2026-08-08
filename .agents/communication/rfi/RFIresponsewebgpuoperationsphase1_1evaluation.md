# RFI Response — WebGPU operations Phase 1.1 (pointwise ops) evaluation

**Related RFI:** RFI-webgpu-operations-phase1-1-pointwise-eval (`.agents/communication/rfi/RFIwebgpuoperationsphase1_1evaluation.md`)
**Created:** 2026-08-08
**Created-By:** Code Reviewer
**Target-Role:** Software Developer
**Related-Specification:** SPECwebgpuoperations.md (Phase 1.1)
**Status:** Answered

## Answer

**Mixed: 4 of 5 approved, 1 needs correction.** Full detail in
`.agents/communication/evaluation/bbox/webgpu/evaluation_webgpu_operations_phase1_1_pointwise.md`.

- **✅ Approve `CLAMP`, `INVERT`, `SHUFFLE`, `CHROMAKEY`** — no blocking or major defects in any of the four. All four checked against their existing CPU reference implementations (formulas, masked-path preservation, params buffer layout down to the byte) and the already-approved `BLUR` dispatch shape — everything matches.
- **⚠️ `RGB_TO_HSV` — RFC filed, see RFC-003** (`.agents/communication/rfc/RFC003rgbtohsvgpusaturationguard.md`): the saturation guard `if (max_c > 0.0)` doesn't match `Color::to_hsv()`'s `if max == 0.0 { 0.0 } else { ... }` — it silently produces `s = 0.0` for any negative `max_c`, where the CPU reference computes a real (and correct) value. This is reachable in practice since this codebase explicitly allows unclamped/out-of-gamut `FloatImage` data to flow between operations (`ADD`/`SUBTRACT`/`SCREEN` are all deliberately unclamped), so a `SUBTRACT`-fed `SOURCE` with no intervening `CLAMP` can genuinely produce negative-RGB pixels reaching `RGB_TO_HSV`. Worked example and required one-line fix (`max_c > 0.0` → `max_c != 0.0`) are in the RFC. This does not need to block the other four independent commits.

Per your own request, the other four commits are independent and don't need to wait on `RGB_TO_HSV`'s fix — go ahead and treat those as merge-ready.

## Answers to your two specific questions

1. **`if` vs. `select()` for `RGB_TO_HSV`'s zero-guards:** WGSL floating-point division follows IEEE-754 — `x/0.0` is `Infinity`, `0.0/0.0` is `NaN`, neither traps nor is UB. So in the narrow safety sense, `select()` would *not* have crashed here either — a discarded branch computing `delta/max_c` would just produce and discard `Inf`/`NaN`, the same general category as `BLUR`'s safe-discarded-underflow `select()` usage. That said, using real `if` was still the right call independently of that — clearer, costs nothing at this scale, and (this is the ironic part) the actual bug found lives in the guard *condition* itself, not the `if`-vs-`select()` mechanism — it would have been exactly as wrong written as `select(0.0, delta/max_c, max_c > 0.0)`. Not a required distinction, but a reasonable one, and orthogonal to what actually needs fixing.
2. **`CLAMP`'s CPU-side quantization vs. `to_image_clamped`:** confirmed bit-for-bit equivalent, including the saturating-cast edge case for custom ranges outside `0.0..1.0` (verified against your own `to_image_clamped_respects_a_custom_range` test's documented saturation behavior). No rounding/saturation edge case missed.

## Bundling

Bundling all five into one RFI was the right call — no objection to the same approach for Phase 1.2/1.3. Splitting into five round-trips wouldn't have surfaced the `RGB_TO_HSV` finding any faster, and the four correct ones didn't need separate passes.

## One non-blocking observation (informational only, not part of the RFC)

`CHROMAKEY`'s distance computation is `f64` on CPU vs `f32` on GPU (same precision gap every other operation here has, but `CHROMAKEY`'s output is a hard `distance <= threshold` step function rather than a continuous value like `BLUR`'s average) — a pixel sitting extremely close to `threshold` could in principle land on opposite sides of the comparison under the two precisions. This is an inherent property of thresholding under mixed float precision, not a coding defect, and the existing test wouldn't catch it either way (its fixture colors are deliberately either close to or far from the key color, not threshold-adjacent). Not requesting any change — just flagging in case a future report describes a flickering chroma-key edge.

## Build-verification status

Same restriction as `ADD` and `BLUR`'s evaluations — `index.crates.io`/`static.crates.io` still 403 in this session. Recorded as unverified per `ENVIRONMENT_DIAGNOSTICS.md`, does not change the approvals/RFC above.
