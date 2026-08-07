# Evaluation: Phase 3 — SUBTRACT consumes bboxes (seventh operation)

**Commit:** d325498 on `claude/bbox-phase-3-subtract`
**Spec:** `1_bboxawarenessspec.md`, Phase 3 section (still held — no resend needed)

## 0. Build-environment note — reproduced independently again

Same blocker as `SCREEN`'s round (unrelated GPU work on real `dev` breaks builds in this sandbox). This time I reconstructed the clean verification chain myself rather than reusing a stale patch: diffed `screen.rs` against the fixed pre-merge commit (`2d2e6f0`, since `origin/dev` has since moved past `SCREEN`'s merge — using `origin/dev` directly would have produced an empty diff), applied it onto the merged `GHOST` branch tip, then diffed and applied `subtract.rs` on top of that. Built and ran the full suite there myself: **279 passed, 0 failed** — matches the report's number exactly, independently reproduced. Diff against real `dev` (`git diff origin/dev...HEAD --stat`) confirms exactly `subtract.rs` changed, one file.

## 1. Summary of the change

`SUBTRACT`'s masked path restricts computation to `intersect(union(Foreground's box, Background's box), Mask's box)` — the same union-based shape `SCREEN` needed last round, for a different algebraic reason.

## 2. Verification against requirements

- **Is the union genuinely required here too?** Verified `subtract_pixels` directly: `subtract(a,b) = a - b` per channel, unclamped. `subtract(a,0) = a` (identity, matches `subtracting_black_is_identity`), but `subtract(0,b) = -b` — a genuine, non-default (and deliberately out-of-gamut, per this operation's own doc comment) result whenever `Background` alone carries real content. So `SUBTRACT` is asymmetric in the same direction as `SCREEN` — not zero-preserving on either input alone — confirming the union is the correct natural box, not a copy-pasted assumption from `SCREEN`'s round. ✅
- **`subtract_single_pixel` vs. `subtract_pixels` equivalence:** identical per-channel formula, one index vs. whole buffer — no window, no risk class. ✅
- **`apply_mask`/`compute_within_bbox` consistency:** both use `Foreground`'s raw pixels as the "original"/pass-through, matching the established convention and `SCREEN`'s own pattern. ✅
- **The report's own load-bearing test:** same shape as `SCREEN`'s — `Foreground` reports an empty box, `Background` carries the only real content in `[3,7)` — and additionally pins down that the restricted result shows a genuine negative value (`< -0.5`) at the pixel where `Background`'s content should have been subtracted in, not just that the two paths match each other (which could pass vacuously if both were wrong in the same way). This is a stronger test than `SCREEN`'s round had, worth calling out. ✅
- **Independent adversarial verification:** reused the same 216-trial brute-force probe shape as `SCREEN`'s round (6×6×6 `Foreground`/`Background`/`Mask` box positions, independent real content on both inputs) — but had to adapt the comparison to compare raw, unclamped `f32` pixel values directly (with a small tolerance) rather than `u8`-quantized output, since `SUBTRACT`'s legitimate negative/out-of-gamut results would otherwise get silently clipped by a `u8` round-trip and could mask a real discrepancy. **216 trials, 0 violations**, run against a build I compiled myself. ✅
- **Diff scope:** confirmed via `git diff origin/dev...HEAD --stat` against real `dev` — exactly `subtract.rs`. ✅

## 3. Issues

None found. No blocking, major, minor, or nit findings.

## 4. What was done well

- Correctly re-derived the union requirement from `SUBTRACT`'s own formula rather than assuming `SCREEN`'s conclusion transfers wholesale — the report explicitly distinguishes *why* the two operations share the union shape (different algebraic source of the asymmetry: multiplicative complement vs. plain linear subtraction), which is exactly the right level of "verify, don't pattern-match" discipline this round has consistently shown since `INVERT`.
- The load-bearing test goes one step further than `SCREEN`'s equivalent by directly asserting the real negative difference appears (not just that restricted equals unrestricted), which rules out both paths being wrong in the same way.
- Proactively disclosed the same build-environment workaround as last round, with the same clear verification trail, making independent reproduction straightforward (once I corrected my own mistake of diffing against a moved `origin/dev` instead of the fixed pre-merge commit).
- Diff confinement, build, and test counts all check out exactly as reported.

## 5. Recommendation

**✅ Approve** — last operation before `hue_key.rs`, the final and by design most complex one in this round.
