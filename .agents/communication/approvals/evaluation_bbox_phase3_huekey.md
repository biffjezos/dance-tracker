# Evaluation: Phase 3 — HUE KEY consumes bboxes (eighth and final operation)

**Branch:** `claude/bbox-phase-3-huekey-real` (based on real `origin/dev` tip `689d0d5`)
**Spec:** `1_bboxawarenessspec.md`, Phase 3 section

## 0. Build-environment note — reproduced independently again

Same `wgpu`-blocked-sandbox situation as `SCREEN`/`SUBTRACT`. Reconstructed the clean verification chain myself: regenerated `screen.rs`'s and `subtract.rs`'s diffs against their respective fixed pre-merge commits, applied both plus `hue_key.rs`'s diff (against real `dev`) in sequence onto the merged `GHOST` branch tip, and built/tested there myself. **283 passed, 0 failed** — matches the report exactly, independently reproduced rather than trusted. Diff against real `dev` confirmed exactly one file: `hue_key.rs`.

## 1. Summary of the change

`HUE KEY`'s masked path restricts computation to `intersect(SOURCE's box, MASK's box)` — structurally identical to `CHROMA KEY`'s pattern. The distinguishing claim: `REFERENCE`'s own reported box plays **no role** in the restriction at all — its raw pixel buffer is always read directly and unrestricted, regardless of `work_area`.

## 2. Verification against requirements — the REFERENCE-box claim specifically

This is the one genuinely new question this operation raises (`CHROMA KEY` had no third input), so I gave it the deepest scrutiny of the four "simple" key/compose operations.

- **Read `key_pixels`/`key_single_pixel` directly:** `target[3]` is always either the constant `0.0` or `src[3]` (`SOURCE`'s own alpha) — `reference[idx]`'s actual value never flows into the output pixel itself, it only selects which of those two constants applies. RGB is always `src`'s RGB unconditionally. So `key_pixels(source=[0,0,0,0], reference=anything)` is `[0,0,0,0]` regardless of `REFERENCE` — the same zero-preservation-on-`SOURCE` shape `CHROMA KEY` has. ✅
- **But *why* is it actually safe to ignore `REFERENCE`'s box, not just "it only picks a branch"?** I worked through the deeper reason rather than accepting the surface explanation: `compute_within_bbox`'s pass-through value at skipped positions is `SOURCE`'s own raw pixel — the same value `apply_mask`'s `original` argument uses. Outside `work_area = intersect(source_box, mask_box)`, one of two things is guaranteed: either outside `source_box` (true keyed output is `[0,0,0,0]` = the pass-through, trivially matching), or outside `mask_box` (`MASK`'s weight is guaranteed `0`, so `apply_mask`'s blend collapses to exactly `original` **regardless of what the "processed" branch would have computed** — including regardless of `REFERENCE`'s contribution to that computation). So `REFERENCE`'s box isn't merely *irrelevant by construction of the formula* — it's irrelevant because `work_area` is already sized so nothing computed outside it can ever reach the final output at all, for any input. This is the same underlying principle that made `BLUR`/`INVERT`/`SHUFFLE`/`CHROMA KEY` all correct, just applied to a third input that happens not to participate in the box calculation.
- **Dimension safety:** confirmed `execute()` validates `SOURCE`/`REFERENCE` dimensions match before any indexed access — no risk of an out-of-bounds read into `reference_pixels` at a mismatched index. ✅
- **Empirical confirmation, not just algebra — the specific test I built for this operation:** held `SOURCE`'s box/pixels, `MASK`'s box/pixels, and `REFERENCE`'s actual pixel *data* all fixed, and varied only what `REFERENCE` *reports* as its box across four deliberately adversarial values (full-frame, empty, and two tiny sub-frame boxes that don't even cover the positions where the real keyed/kept hue data differs). All four produced **bit-identical output**, matching the fully-unrestricted baseline too. This directly tests the claim empirically rather than accepting the algebra alone — if the implementation secretly consulted `REFERENCE`'s box anywhere, this test would have caught it. ✅
- **Broader consume-equivalence brute force:** 36 combinations of `SOURCE`/`MASK` box positions (6×6, matching the `CHROMA KEY`-style grid), randomized hue/color data — 0 violations. ✅
- **Test count:** 11 `#[test]` functions in the file, matching the claimed "7 pre-existing + 4 new." ✅
- **Diff scope:** confirmed via `git diff origin/dev...HEAD --stat` — exactly `hue_key.rs`. ✅

## 3. Issues

None found. No blocking, major, minor, or nit findings. This closes out Phase 3 cleanly.

## 4. What was done well

- Correctly identified and precisely scoped the one genuinely new question this operation poses (a third, non-`Mask` input that participates in the keying formula but shouldn't participate in the box restriction) rather than mechanically copying `CHROMA KEY`'s diff.
- The explanation for why `REFERENCE`'s box doesn't matter is accurate as far as it goes ("it only decides which branch"), and my own deeper trace confirms the fuller reason (work_area's construction already makes anything computed outside it unreachable in the final output, for any input) — the report's practical conclusion is right even though I found a more fundamental justification for it.
- Consistent, disciplined re-verification of zero-preservation from the operation's own formula at every step of this eight-operation series (never assumed a prior operation's shape transfers), which is exactly the practice that caught real bugs in `BLUR` and self-caught one in `INVERT`/`SCREEN`-adjacent reasoning earlier in this round.
- Transparent build-environment disclosure, consistent with the last two rounds, that made independent reproduction (once I reconstructed the correct patch chain) straightforward.

## 5. Recommendation

**✅ Approve** — this completes Phase 3. All eight operations (`BLUR`, `INVERT`, `SHUFFLE`, `CHROMA KEY`, `GHOST`, `SCREEN`, `SUBTRACT`, `HUE KEY`) have now been migrated and independently verified, with one real bug caught and fixed along the way (`BLUR`'s missing radius-growth in the work-area calculation) and every other operation's distinguishing safety argument checked from first principles rather than pattern-matched.
