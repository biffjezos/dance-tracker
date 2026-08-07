# Evaluation: Phase 3 — GHOST consumes bboxes (fifth operation)

**Commit:** 206da18 on `claude/bbox-phase-3-ghost`
**Spec:** `1_bboxawarenessspec.md`, Phase 3 section (still held — no resend needed)

## 1. Summary of the change

`GHOST` breaks from the `apply_mask`-based pattern every prior operation used. Only its `cutout_pixels` step (genuinely per-pixel, zero-preserving) is restricted to `intersect(SOURCE's box, MASK's box)`; the translate/composite loop — where a ghost's content can land anywhere up to `GHOST_COUNT * DISTANCE * (SPATIAL_X, SPATIAL_Y)` pixels away — is deliberately left full-frame. The skip-substitute value is literal `[0,0,0,0]`, not `SOURCE`'s raw pixel (unlike every prior operation), because `GHOST` has no final `apply_mask` blend to fall back on.

## 2. Verification against requirements

This is the first operation where the safety argument doesn't reduce to "the blend at the end erases whatever we skip" — the substitute value feeds directly into the final visual output via `composite_over`. That's a materially different (and riskier) claim than the previous four operations made, so I gave it correspondingly deeper scrutiny: read every function in the affected pipeline, verified the safety claim algebraically against the actual formula, then built an adversarial end-to-end test the report's own tests don't cover.

- **Is the substitution actually safe?** The report's claim: a zero-alpha cutout pixel is "visually inert downstream regardless of its RGB" because `composite_over` discards RGB whenever `fg_a=0`. I read `composite_over`'s exact formula:
  ```rust
  out_a = fg_a + bg_a * (1.0 - fg_a);
  out_px[c] = if out_a > 0.0 { (fg_px[c]*fg_a + bg_px[c]*bg_a*(1.0-fg_a)) / out_a } else { 0.0 };
  ```
  With `fg_a = 0`: the `fg_px[c]*fg_a` term is exactly `fg_px[c] * 0 = 0` for any finite `fg_px[c]` (IEEE-754 exact, not approximate), so `out_px[c]` reduces to either `bg_px[c]` (when `bg_a>0`) or `0.0` (when both are 0) — **independent of `fg_px[c]`'s actual value in every case.** The claim is correct, and it's an exact algebraic identity, not an approximation. ✅
  - I then traced every place the cutout buffer is actually consumed (`record_history` → `delayed_cutout` → `translate_pixels` → `composite_over`, and the direct `show_source` path) to confirm `composite_over` really is the *only* place cutout values get used arithmetically — `translate_pixels` is a pure spatial copy with no arithmetic on pixel values, so it can't reintroduce a dependency on the substituted RGB. Confirmed. ✅
- **Precondition safety split (Case A vs. Case B):** outside `SOURCE`'s box, the true cutout is exactly `[0,0,0,0]` (RGB copies a guaranteed-zero source, alpha multiplies by zero source-alpha) — substitution matches exactly, no argument needed. Outside `MASK`'s box but *inside* `SOURCE`'s box, the true cutout is `[real_RGB, 0]` (mask-alpha is guaranteed zero there, source RGB can be anything) — this is the case that actually needs the `composite_over` argument, and it's the case none of the report's own `consume_equivalence_*` tests exercise with a *non-degenerate* background (both use `ghost_count: 0`, so `composite_over`'s background is always `[0,0,0,0]` too, meaning the `bg_a>0` branch of the formula — the one that actually matters for the safety argument — is never hit by their own tests).
- **Adversarial gap-filling test, run against the real code, not modeled:** I built a scenario the existing tests can't reach — `SOURCE` has real green content at `x=2` (inside `MASK`'s box, genuinely computed) and real red content at `x=5` (outside `MASK`'s box, so its cutout gets the `[0,0,0,0]` substitute under restriction). `GHOST_COUNT=1`, `DISTANCE=3`, `SPATIAL_X=1` translates the opaque green cutout from `x=2` to land exactly at `x=5` — the same position as the red/substitute mismatch — so `show_source`'s final `composite_over` call there genuinely hits the `bg_a>0` branch with a real, differently-colored background underneath a zero-alpha (but RGB-mismatched) foreground. Ran both the restricted and unrestricted paths: **identical output**, and `x=5` correctly shows the translated green ghost, never red. This is the strongest possible confirmation of the report's central safety claim — not just algebra, but the actual pipeline under the exact adversarial condition designed to break it. ✅
- **Is the translate/composite loop genuinely untouched (not accidentally also restricted)?** Confirmed via diff — no bbox logic anywhere in `render_with_cutout`. The report's own `a_ghost_translated_outside_masks_own_box_still_renders_correctly` test directly proves this (a ghost offset well outside `MASK`'s tight box still renders as real, opaque content) — I consider this the load-bearing test for the operation and it's well-constructed. ✅
- **Is `render()`'s public signature/behavior genuinely unchanged** (so all 19 pre-existing tests, many calling it directly, still pass unmodified)? Confirmed via diff — no hunk touches `pub fn render`; it's untouched, only newly factored-out code was added alongside it. ✅
- **Diff scope:** confirmed via `git show --stat` — exactly `ghost.rs`, one file. ✅
- **Build/test:** ran myself on the pristine branch — 271 passed, 0 failed, matching the report exactly (272 with my own added probe test on top). ✅

## 3. Issues

None found. No blocking, major, minor, or nit findings. This is the most structurally demanding operation migrated so far, and the reasoning holds up under the deepest scrutiny I've applied in this review series.

## 4. What was done well

- Correctly recognized that `GHOST` doesn't fit the established recipe rather than forcing it — the decision to leave the translate/composite loop untouched (and explicitly scope that as future work, not attempted here) is the right call given the complexity/risk tradeoff, and is honestly reported as a real limitation (this round doesn't optimize `GHOST`'s dominant cost) rather than glossed over.
- The `[0,0,0,0]` substitution reasoning is a genuinely more sophisticated argument than any prior operation needed, and it's correct — verified independently both algebraically and with an end-to-end adversarial test built specifically to exercise the one code path (`composite_over`'s `bg_a>0` branch under a zero-alpha, RGB-mismatched foreground) that the implementor's own tests happen not to reach.
- `render_with_cutout`'s factoring is minimal and surgical — it isolates exactly the reusable logic without touching `render()`'s own contract, which is why all 19 pre-existing tests needed zero changes.
- The load-bearing regression test (`a_ghost_translated_outside_masks_own_box_still_renders_correctly`) targets precisely the failure mode this operation's design is most at risk of (accidentally clipping translated ghost content), not a generic sanity check.

## 5. Recommendation

**✅ Approve**
