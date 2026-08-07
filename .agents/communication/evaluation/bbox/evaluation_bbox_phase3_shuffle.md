# Evaluation: Phase 3 — SHUFFLE consumes bboxes (third operation)

**Commit:** ad38bf4 on `claude/bbox-phase-3-shuffle`
**Spec:** `1_bboxawarenessspec.md`, Phase 3 section (still held — no resend needed)

## 1. Summary of the change

`SHUFFLE`'s masked path restricts computation to `intersect(SOURCE's box, MASK's box)` — no growth, since `SHUFFLE` reads only the pixel it writes (no neighbor window, unlike `BLUR`). No `output_bbox()` override added.

## 2. Verification against requirements

Continuing the same scrutiny that caught the `BLUR` bug and confirmed `INVERT`'s self-correction: checked whether `SOURCE`'s raw box is actually a valid intersection operand here, rather than accepting the report's zero-preservation claim on trust.

- **Is `SHUFFLE` genuinely zero-preserving, for *every* channel mapping, not just the default identity one?** Read `channel_value` directly: each output channel is either `pixel[i]` (a straight copy of one of the source's 4 channels) or `T::default()` (`Off`, hardcoded zero) — there is no third case, and no arithmetic. So for a source pixel `[0,0,0,0]`, every possible `(red, green, blue, alpha)` configuration produces either a copy of a zero channel or a hardcoded zero — always `[0,0,0,0]`. This is true by the code's structure for *any* configuration, not empirically true only for the mappings someone happened to test. ✅
- **Diff scope:** confirmed via `git show --stat` — exactly `shuffle.rs`, one file. ✅
- **Build/test:** ran myself — 262 passed, 0 failed, matching the report exactly. ✅
- **`work_area` formula:** `intersect(find_bbox(Source), find_bbox(Mask))`, both falling back to full-frame, no growth — matches the reasoning and is the correct instance of the same general rule established across `BLUR`/`INVERT` (operation's own natural bbox = `SOURCE`'s box here, since no neighbor pixels are ever read). ✅
- **Independent adversarial verification, going one step further than the report's own tests:** all of the report's new tests use either the default identity mapping or (in the sub-frame-`SOURCE` test) a single non-identity swap (`RED ← BLUE`). I wrote a broader brute-force probe: 6 `SOURCE` box positions × 6 `MASK` box positions × 4 channel-mapping configurations (identity, all-`Off`, full 4-way swap, mixed with `Off`) — **144 trials, 0 violations**, comparing the restricted path against the full-frame ground truth. This directly stress-tests the "for *any* mapping" part of the zero-preservation claim, not just the specific mappings already in the test file. ✅
- **No `output_bbox()` override:** confirmed directly (none present) — correct, since `SHUFFLE`'s own natural box already equals whatever `SOURCE`'s box is, with no independent tightening to report. ✅
- **Test comment accuracy:** unlike the `INVERT` round's stale/backwards comment, this round's `consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box` test comment accurately describes the zero-preservation reasoning and matches the code. ✅

## 3. Issues

None found. No blocking, major, minor, or nit findings.

## 4. What was done well

- Correctly re-derived `SHUFFLE`'s own zero-preservation property from first principles (reading `channel_value`'s actual cases) rather than assuming either `BLUR`'s or `INVERT`'s pattern transfers — this is the third operation in a row where the report explicitly re-checks this instead of pattern-matching on the previous one, which is exactly the discipline the `INVERT` bug should have taught.
- Went a step further than `INVERT`'s round by exercising a non-default channel mapping (`RED ← BLUE`) in the sub-frame-`SOURCE` regression test itself, not just the default identity mapping — a stronger test than strictly necessary, which is why my own adversarial probe (144 combinations across 4 distinct mappings) came back clean rather than surfacing anything new.
- No leftover inaccurate comments this round.
- Diff confinement, build, and test counts all check out exactly as reported.

## 5. Recommendation

**✅ Approve**
