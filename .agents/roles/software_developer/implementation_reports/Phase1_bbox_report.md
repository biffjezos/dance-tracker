---
---
# Implementation Report: Phase 1 — Geometric producers report (RESIZE, MOVE)

**Branch:** `claude/bbox-phase-1` (pushed, PR not yet opened — awaiting evaluator re-review)
**Commit:** `f85244c` (fixes `bfe93c5`'s evaluator-caught blocking bug)
**Spec:** Bounding-box awareness spec, Phase 1 section — per `BBOX_CONVENTIONS.md`

## Summary

`RESIZE` and `MOVE` both override `output_bbox()` to remap their
`Source` input's reported box through their own parameters — pure
arithmetic, no pixel reads, report-only. Neither operation's `execute()`
changed. **This revision fixes one blocking bug the evaluator caught in
the previous round** (see "Fix applied" below).

## Fix applied (blocking, from evaluator review of commit `bfe93c5`)

**Bug:** `MOVE::output_bbox()` rounded `OFFSET_X`/`OFFSET_Y` to the
nearest integer *before* translating the box (`self.offset_x.round() as
i32`), but `move_pixels` — the actual pixel-sampling code — uses the
exact, unrounded, truncating-sample offset. These two roundings disagree
whenever the offset has a fractional part, and the reported box could
land strictly smaller than the true content extent — a direct violation
of `BBOX_CONVENTIONS.md`'s core safety invariant ("must never be smaller
than the true extent of non-default content").

The evaluator's own counter-example: width=4, `Source` box `[0,1)`
(only source pixel 0 real), `offset_x=0.4`. `move_pixels` puts real
content at `dest_x=1` (`src_x = 1 - 0.4 = 0.6`, truncates to source
pixel 0). The old code reported `offset_x.round() = 0`, giving box
`[0,1)` — missing `dest_x=1` entirely.

**Fix:** round the *translated continuous bound* outward (floor the
lower edge, ceil the upper edge) instead of rounding the offset first —
the same outward-rounding pattern `RESIZE`'s `output_bbox()` already
uses correctly in the same commit:
```rust
let translated = Rect {
    x0: (source_box.x0 as f64 + self.offset_x).floor() as i32,
    y0: (source_box.y0 as f64 + self.offset_y).floor() as i32,
    x1: (source_box.x1 as f64 + self.offset_x).ceil() as i32,
    y1: (source_box.y1 as f64 + self.offset_y).ceil() as i32,
};
```
Added `output_bbox_with_a_fractional_offset_never_undershoots_move_pixels_real_content`
— a regression test using the evaluator's exact counter-example, plus a
second cross-check that directly runs `move_pixels` and asserts the
reported box covers every dest pixel that actually received real
content, not just the one hand-picked case.

All prior integer-offset tests still pass unchanged, since `floor`/`ceil`
of an already-integer value is a no-op.

## What was implemented (unchanged from prior round otherwise)

- **`engine/src/operations/transform/resize.rs`**: `output_bbox()`
  remaps the `Source` box through the exact inverse of `resize_pixels`'s
  own center-relative `dest -> src` mapping, rounded outward, intersected
  with `Rect::full`. (No changes this round — evaluator's brute-force
  check across 5,304 combinations found zero violations.)
- **`engine/src/operations/transform/move_op.rs`**: `output_bbox()`
  translates the `Source` box by `(OFFSET_X, OFFSET_Y)` (now via the
  outward-rounding fix above), intersected with `Rect::full` — an offset
  that moves the whole box off-canvas collapses to `Rect::empty()`.

## Tests

All from the prior round, plus the new regression test:

| Test | Verifies |
|---|---|
| `output_bbox_at_50_percent_on_a_full_frame_input_is_exactly_half_centered` (resize) | AC1 |
| `output_bbox_at_100_percent_on_a_full_frame_input_stays_full_frame` (resize) | identity sanity check |
| `output_bbox_with_no_reported_source_box_defaults_to_full_frame` (resize) | unwired/not-opted-in fallback |
| `chaining_resize_into_an_unmodified_invert_is_still_pixel_identical` (resize) | AC3 |
| `output_bbox_translates_a_full_frame_input_by_exactly_the_offset_clamped_to_the_frame` (move) | AC2 |
| `output_bbox_with_an_offset_larger_than_the_frame_is_empty_not_negative_extent` (move) | AC2 |
| **`output_bbox_with_a_fractional_offset_never_undershoots_move_pixels_real_content` (move)** | **new — regression for the fixed blocking bug** |
| `output_bbox_with_no_reported_source_box_defaults_to_full_frame_then_translates` (move) | unwired/not-opted-in fallback |
| `chaining_move_into_an_unmodified_invert_is_still_pixel_identical` (move) | AC3 |

**Results:**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 239 passed, 0 failed, 0 ignored (238 prior + 1 new
                regression test)
```

## Acceptance criteria checklist

1. ✅ `RESIZE` at 50% on a full-frame input reports a box exactly half
   the frame's width/height, centered.
2. ✅ `MOVE` with a nonzero offset on a full-frame input reports a box
   translated by exactly that offset, clamped to the frame bounds; an
   offset larger than the frame reports `Rect::empty()` — **now also
   verified safe for fractional offsets**, closing the gap the evaluator
   found.
3. ✅ Chaining `RESIZE`/`MOVE` into an unmodified downstream operation
   (`INVERT`) still produces pixel-identical output.

## Diff scope

Confined to exactly `engine/src/operations/transform/resize.rs` and
`engine/src/operations/transform/move_op.rs` across both commits in
this branch (`bfe93c5` + the fix `f85244c`). No other file touched.

## Evaluator comments not otherwise addressed

None — the evaluator's single blocking finding is the fix above; no
major/minor/nit findings were raised.

## Status

Branch `claude/bbox-phase-1` is pushed to origin (commit `f85244c`).
Resubmitting for evaluator re-review before opening a PR or proceeding
to Phase 2.
