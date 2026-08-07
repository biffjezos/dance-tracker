# Evaluation: Phase 3 — INVERT consumes bboxes (second operation)

**Commit:** 3618899 on `claude/bbox-phase-3-invert`
**Spec:** `1_bboxawarenessspec.md`, Phase 3 section (still held from earlier — no resend needed)

## 1. Summary of the change

`INVERT`'s masked path restricts computation to `MASK`'s own reported box **alone** — deliberately not intersected with `SOURCE`'s box, unlike `BLUR`. The report's stated reason: `INVERT` isn't zero-preserving (`invert([0,0,0,0]) = [1,1,1,1]`), so `SOURCE`'s "no real content here" box says nothing about where `INVERT`'s own output is non-default. No `output_bbox()` override is added, matching that `INVERT`'s true natural box is already `Rect::full`.

## 2. Verification against requirements

Given the `BLUR` bug I caught last round, I checked whether the same class of mistake — using the wrong "natural bbox" operand in the `work_area` intersection — could recur here, rather than accepting the report's self-diagnosis at face value.

**Is the design actually correct, not just self-consistent?** Yes — and it's a correct instance of the *same* general rule `BLUR`'s fix established, not a special case invented for `INVERT`. The rule: `work_area = intersect(mask_box, operation's own natural bbox)`, where "operation's own natural bbox" is whatever `output_bbox()` would report (or *would* report if it existed) — the region where the operation's own unmasked output can be non-default. For `BLUR`, that's `grow(source_box, radius)`. For `INVERT`, since inverting `[0,0,0,0]` yields `[1,1,1,1]` (confirmed directly: `invert_pixels` maps `1.0 - channel` over all 4 channels including alpha, so this claim isn't asserted, it's read off the actual formula), the operation's own natural bbox is unconditionally `Rect::full` — which is exactly why `INVERT` never overrode `output_bbox()` in Phase 1/2 in the first place. `intersect(mask_box, full) == mask_box`, so "mask box alone" isn't a deviation from the `BLUR` recipe, it's what the recipe reduces to when an operation's natural bbox is already full-frame. This is real design consistency, not two different rules for two operations.

- **Diff scope:** confirmed via `git show --stat` — exactly `invert.rs`, one file. ✅
- **Build/test:** ran myself — 258 passed, 0 failed, matching the report exactly. ✅
- **No `output_bbox()` override added:** confirmed directly (no `fn output_bbox` in the diff). Correct, per the reasoning above. ✅
- **Per-pixel closure correctness:** `compute_within_bbox`'s closure (`[1.0 - pixels[idx], ...]`) is a direct, windowless per-pixel formula — unlike `BLUR`'s two-pass-vs-single-pass equivalence question, there's no possibility of a normalization/windowing mismatch here, since the closure reads exactly the same single pixel `invert_pixels` would for that index. Lower-risk by construction, not just by luck.
- **Independent adversarial verification:** wrote my own brute-force test (not reusing theirs) — 64 combinations of `SOURCE` box position × `MASK` box position (edges, single pixels, full-frame, disjoint/overlapping combinations), each with randomized pixel data respecting the precondition that a reported box's exterior is genuinely `[0,0,0,0]`, comparing the restricted path against the full-frame ground truth. **0 violations.** This is the same rigor that caught the `BLUR` bug, applied here and coming back clean. ✅
- **AC1–3:** all three tests present and structurally sound, matching the pattern established for `BLUR`. The load-bearing one (`consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box`) is exactly the test shape that caught `BLUR`'s bug, and the report is credible that it caught an equivalent mistake here before review — self-correction working as intended.

## 3. Issues

**Minor:**
- The comment inside `consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box` says: *"Unlike BLUR, INVERT is a pure per-pixel op with no spreading, so its natural box is exactly SOURCE's own reported box (no growth needed)."* This is backwards from both the actual code and the `execute()` comment directly above it in the same file, which correctly states `INVERT`'s natural box is `Rect::full`, **independent of `SOURCE` entirely** (not "`SOURCE`'s own box, just without growth"). The code and the assertion are correct — only this one comment mischaracterizes *why* the test passes. Left as-is, a future reader skimming just this test (without re-reading `execute()`'s own reasoning) could come away thinking `work_area` is `intersect(source_box, mask_box)` with no growth, when it's actually `mask_box` alone with `SOURCE` playing no role at all — which would be actively misleading when this pattern gets reused for the next operation. Worth a one-line fix before merge: replace "its natural box is exactly SOURCE's own reported box" with "its natural box is Rect::full, independent of SOURCE" (or simply delete the incorrect clause and keep the accurate "no growth needed" framing).

No blocking or major issues found.

## 4. What was done well

- Caught and fixed the `BLUR`-equivalent mistake *before* this reached review, using the same regression-test shape that caught it last time — this is exactly the right response to prior feedback: generalizing the lesson ("check zero-preservation before assuming the recipe transfers") rather than just patching the one flagged instance.
- The report's own reasoning about zero-preservation is correct and, more importantly, is a genuine instance of the general `work_area = intersect(mask_box, natural_bbox)` principle rather than an ad-hoc carve-out — I verified this generalization holds, not just took the report's framing on trust.
- Explicitly flagged the plan to re-verify this same property (zero-preservation / natural-bbox shape) per-operation for the rest of the migration queue rather than assuming one pattern fits all eight — the right level of caution given what this round already found twice.

## 5. Recommendation

**⚠️ Approve with minor comments** — ready to merge; fix the one inaccurate test comment (or leave it for a trivial follow-up) since it doesn't affect correctness, only a future reader's understanding of *why* the test passes.
