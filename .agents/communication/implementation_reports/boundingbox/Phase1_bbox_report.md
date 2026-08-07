# Implementation Report: Phase 1 — Geometric producers report (RESIZE, MOVE)

**Branch:** `claude/bbox-phase-1` (pushed, PR not yet opened — awaiting evaluator per the phased cycle)
**Commit:** `bfe93c5`
**Spec:** Bounding-box awareness spec, Phase 1 — per `BBOX_CONVENTIONS.md`

## Summary

`RESIZE` and `MOVE` (both now landed in `dev`) override `output_bbox()`
to remap their `Source` input's reported box through their own
parameters — pure arithmetic, no pixel reads, report-only. Neither
operation's `execute()` changed.

## What was implemented

- **`engine/src/operations/transform/resize.rs`**: `output_bbox()`
  remaps the `Source` box through the exact inverse of `resize_pixels`'s
  own center-relative `dest -> src` mapping:
  `dest = cx + (src - cx) * (scale / 100)`. Rounds outward — `floor` the
  lower edge, `ceil` the upper edge — so the reported box is never
  smaller than `resize_pixels`'s true content extent, per
  `BBOX_CONVENTIONS.md`'s "larger is safe, smaller is not" invariant.
  Result is intersected with `Rect::full` — at `scale > 100` the
  unclamped remap can exceed the frame, but `resize_pixels` itself never
  writes outside it, so anything past the frame edge is never real
  content. Falls back to `Rect::full(ctx.meta.width, ctx.meta.height)`
  when `Source` reported nothing (unwired, or an upstream operation that
  hasn't opted in yet).
- **`engine/src/operations/transform/move_op.rs`**: `output_bbox()`
  translates the `Source` box by `(OFFSET_X, OFFSET_Y)` (rounded to the
  nearest pixel), then intersects with `Rect::full` — an offset that
  moves the whole box off-canvas collapses to `Rect::empty()` via that
  intersection, never a box with negative extent. Same unwired
  fallback as `RESIZE`.
- **No `execute()` change in either file.** Both operations' pixel
  output is untouched — confirmed by diffing against the pre-Phase-1
  `execute()` bodies directly.

## Tests

| Test | Verifies |
|---|---|
| `output_bbox_at_50_percent_on_a_full_frame_input_is_exactly_half_centered` (resize) | **AC1** — an 8×8 full-frame input at 50% reports `Rect{2,2,6,6}` — exactly half the frame, centered |
| `output_bbox_at_100_percent_on_a_full_frame_input_stays_full_frame` (resize) | sanity check — identity scale is a no-op on the box too |
| `output_bbox_with_no_reported_source_box_defaults_to_full_frame` (resize) | the unwired/not-yet-opted-in fallback |
| `chaining_resize_into_an_unmodified_invert_is_still_pixel_identical` (resize) | **AC3** — `RESIZE → INVERT` (unmodified) through a real `Graph`/`PreviewExecutor`, pixel output hand-verified against `INVERT`'s own math |
| `output_bbox_translates_a_full_frame_input_by_exactly_the_offset_clamped_to_the_frame` (move) | **AC2** — an 8×8 full-frame box offset by `(2,1)` reports `Rect{2,1,8,8}` (clamped) |
| `output_bbox_with_an_offset_larger_than_the_frame_is_empty_not_negative_extent` (move) | **AC2** — offset `(100,0)` on an 8×8 frame reports `Rect::empty()`, not a negative-extent rect |
| `output_bbox_with_no_reported_source_box_defaults_to_full_frame_then_translates` (move) | the unwired/not-yet-opted-in fallback, then translated |
| `chaining_move_into_an_unmodified_invert_is_still_pixel_identical` (move) | **AC3** — `MOVE → INVERT` (unmodified) through a real `Graph`/`PreviewExecutor`, pixel output hand-verified |

**Results:**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 238 passed, 0 failed, 0 ignored (230 baseline + 8 new
                this phase: 4 in resize.rs, 4 in move_op.rs)
```

## Acceptance criteria checklist

1. ✅ `RESIZE` at 50% on a full-frame input reports a box exactly half
   the frame's width/height, centered.
2. ✅ `MOVE` with a nonzero offset on a full-frame input reports a box
   translated by exactly that offset, clamped to the frame bounds; an
   offset larger than the frame reports `Rect::empty()`.
3. ✅ Chaining `RESIZE`/`MOVE` into an unmodified downstream operation
   (`INVERT`) still produces pixel-identical output — verified via two
   real-graph integration tests (one per operation), each with
   hand-derived expected pixels, not just self-consistency.

## Diff scope

Confined to exactly the two files the phase names:
`engine/src/operations/transform/resize.rs` and
`engine/src/operations/transform/move_op.rs` (confirmed via
`git status --short`). No other operation touched, no executor/core
`compositor/` file touched this phase (that was Phase 0).

## Out of scope this phase (per spec, untouched)

- `BLUR`'s box-growing (Phase 2).
- Any operation consuming `ctx.input_bboxes` to skip real work (Phase 3).
- Content-derived boxes (chromakey/hue_key) — excluded from this round
  entirely, per `PARKED_WORK.md`.

## Status

Branch `claude/bbox-phase-1` is pushed to origin. Per the agreed cycle,
no PR has been opened yet — awaiting the evaluator's report on this
phase before proceeding to Phase 2 (`BLUR` growing its reported box).
