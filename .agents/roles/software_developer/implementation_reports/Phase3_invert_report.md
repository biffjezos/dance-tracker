---
---
# Implementation Report: Phase 3 — INVERT consumes bboxes (second operation)

**Branch:** `claude/bbox-phase-3-invert` (pushed, PR not yet opened — awaiting evaluator per the phased cycle)
**Commit:** `3618899`
**Spec:** Bounding-box awareness spec, Phase 3 section — per `BBOX_CONVENTIONS.md`

## Summary

`INVERT` is the second of eight operations migrated to consume boxes.
With a wired `MASK`, `execute()` restricts the actual invert
computation to `MASK`'s own reported box — **deliberately not
intersected with `SOURCE`'s own box**, unlike `BLUR`. This is the
important finding this round: applying `BLUR`'s recipe unmodified to
`INVERT` is actively wrong, and my own regression test caught it before
this ever reached review.

## Why `INVERT`'s work area is different from `BLUR`'s

`BLUR` is zero-preserving: a box-averaging window over all-default
(`[0,0,0,0]`) neighbors stays `[0,0,0,0]`, so `SOURCE`'s reported box
(where its content is genuinely non-default) is a valid bound on where
`BLUR`'s own output can be non-default too — modulo the radius-growth
Phase 2 already accounts for.

`INVERT` is **not** zero-preserving: `invert([0,0,0,0]) = [1,1,1,1]`,
fully opaque white — very much non-default. A "`SOURCE` has no real
content here" box says nothing about where `INVERT`'s own output is
non-default, because `INVERT`'s output is generically non-default
*everywhere* regardless of `SOURCE`. This is exactly why `INVERT` never
overrode `output_bbox()` in Phase 1/2 — its own true natural box
(ignoring `MASK` entirely) already is `Rect::full`, the same default
every un-migrated operation inherits.

The only valid restriction for `INVERT`, then, is `MASK`'s own reported
box alone: outside it, `MASK`'s weight is guaranteed zero (by
`mask_box`'s own definition), so `apply_mask` discards whatever
"processed" value we hand it there regardless of correctness — safe to
copy through the original. *Inside* `mask_box`, `apply_mask` may use
the processed value at any weight, so every pixel there needs a
genuinely-computed inversion, independent of what `SOURCE`'s own box
says.

**I initially wrote this the same way as `BLUR`** (intersecting
`SOURCE`'s box with `MASK`'s box) and caught the mistake myself via the
`consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box`
test, which failed immediately: with `SOURCE` real content confined to
`[3,7)` on a 10-wide frame and `MASK` fully opaque everywhere, the
buggy version left pixels `x=0..3` and `x=7..10` as raw, un-inverted
`[0,0,0,0]` instead of the correct fully-inverted `[255,255,255,255]`
`apply_mask` (weight=1 everywhere) should have shown. Fixed before this
ever reached the evaluator.

## What was implemented

- **`engine/src/operations/transform/invert.rs`**: `execute()`'s masked
  path now computes `work_area = find_bbox(&ctx.input_bboxes,
  Input::Mask).unwrap_or_else(|| Rect::full(...))` — no intersection
  with `Input::Source`'s box — then calls `compute_within_bbox` with a
  closure computing `1 - channel` directly per pixel (no window, unlike
  `BLUR` — `INVERT` was already trivially per-pixel). Without a `MASK`,
  the original full-frame `invert_pixels` path is used unchanged.
- No `output_bbox()` override added — `INVERT` correctly keeps
  inheriting the full-frame default, per the reasoning above.

## Tests

All 6 pre-existing tests pass unchanged, plus:

| Test | Verifies |
|---|---|
| `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` | AC1 |
| **`consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box`** | **the load-bearing test — caught the zero-preservation bug described above before it ever reached review** |
| `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` | AC2 — 1 pixel computed vs. 16 for a full-frame box |
| `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` | AC3 |

**Results:**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 258 passed, 0 failed, 0 ignored (254 baseline + 4 new)
```

## Acceptance criteria checklist (per INVERT)

1. ✅ Consume-equivalence, including the sub-frame-`SOURCE` case that
   caught the real bug.
2. ✅ Instrumentation: strictly fewer pixels computed with a smaller
   mask box.
3. ✅ Graph-level integration: `CHECKERBOARD → RESIZE → MOVE` wired into
   `INVERT`'s `MASK` produces pixel-identical output with bbox
   consumption on vs. off.

## Diff scope

Confined to exactly `engine/src/operations/transform/invert.rs`
(confirmed via `git status --short`). No other file touched.

## A note for the remaining operations

Given what this round found, I'll explicitly check each remaining
operation's own zero-preservation property before assuming `BLUR`'s
"intersect with `SOURCE`'s (possibly grown) box" recipe applies — some
(`shuffle`, `add`/`screen`/`subtract` when their *other* input carries
real content, `ghost`) may turn out to need `INVERT`'s "MASK's box
alone" shape instead, and I'll verify each with the same sub-frame-
`SOURCE` regression-test pattern before calling it done, not assume the
first operation's pattern generalizes.

## Status

Branch `claude/bbox-phase-3-invert` is pushed to origin. Per the agreed
cycle, no PR has been opened yet — awaiting the evaluator's report on
this operation before proceeding to the next one (`shuffle.rs`).
