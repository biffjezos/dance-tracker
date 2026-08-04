---
---
# Implementation Report: Phase 2 — BLUR grows (report-only)

**Branch:** `claude/bbox-phase-2` (pushed, PR not yet opened — awaiting evaluator per the phased cycle)
**Commit:** `2bab31a`
**Spec:** Bounding-box awareness spec, Phase 2 section — per `BBOX_CONVENTIONS.md`

## Summary

`BLUR` overrides `output_bbox()` to grow its `Source` input's reported
box by its own kernel radius on every side (via `Rect::grow`), clamped
to the frame. Report-only — `execute()` is unchanged.

## Why growing by radius is the correct (and safe) box

`blur_pixels`'s box kernel of width `2 * radius_px + 1` means any output
pixel within `radius_px` of a real (non-default) source pixel can pull
that pixel into its own averaging window and produce non-default
output. So the true content extent can spread up to `radius_px` pixels
beyond the source's own reported box on every side — exactly what
`Rect::grow(radius_px)` computes. Given Phase 1's lesson (the `MOVE`
rounding bug), I double-checked there's no rounding subtlety here:
`radius_px` is a whole-pixel `u32` already (the parameter's own stepper
enforces integer steps), and `grow`'s arithmetic is exact integer
addition/subtraction — no continuous-to-discrete rounding step exists
in this operation the way `RESIZE`'s scale-based remap or `MOVE`'s
fractional offset did.

## What was implemented

- **`engine/src/operations/transform/blur.rs`**: `output_bbox()` reads
  `Source`'s reported box (falling back to `Rect::full` if unwired or
  the upstream operation hasn't opted in), returns `Rect::empty()`
  immediately if that box is already empty, otherwise grows it by
  `self.radius_px as i32` and intersects the result with
  `Rect::full(ctx.meta.width, ctx.meta.height)`.
- **No `execute()` change.** `blur_pixels` itself is untouched.

## Tests

| Test | Verifies |
|---|---|
| `output_bbox_grows_a_sub_frame_box_by_exactly_the_radius_clamped_to_the_frame` | **AC1** — a sub-frame box grown by radius=2, well within the frame (no clamp needed) |
| `output_bbox_growth_past_the_frame_edge_is_clamped` | **AC1** — growth that would exceed the frame is clamped, not left negative/over-wide |
| `output_bbox_of_an_already_full_frame_source_stays_full_frame_after_growth` | **AC2** — grow-then-clamp is a no-op at the frame edge |
| `output_bbox_with_no_reported_source_box_defaults_to_full_frame_then_grows_and_clamps` | unwired/not-yet-opted-in fallback |
| `chaining_blur_into_an_unmodified_invert_is_still_pixel_identical` | same AC3-style check Phase 1 used — `BLUR → INVERT` (unmodified) through a real `Graph`/`PreviewExecutor`, pixel output hand-verified against both `blur_pixels`'s and `invert_pixels`'s actual math |

**Results:**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 244 passed, 0 failed, 0 ignored (239 baseline + 5 new
                this phase)
```

## Acceptance criteria checklist

1. ✅ Given a `Source` reporting a sub-frame box, `BLUR`'s reported box
   is that box grown by exactly its radius on every side, clamped so it
   never exceeds `Rect::full`.
2. ✅ A `Source` already reporting full-frame stays full-frame after
   growth (grow-then-clamp is a no-op at the frame edge).

## Diff scope

Confined to exactly `engine/src/operations/transform/blur.rs`
(confirmed via `git status --short`). No other file touched.

## Out of scope this phase (per spec, untouched)

- Any operation consuming `ctx.input_bboxes` to skip real work (Phase 3
  — `blur.rs`, `invert.rs`, `shuffle.rs`, `chromakey.rs`, `hue_key.rs`,
  `ghost.rs`, `add.rs`, `screen.rs`, `subtract.rs`).
- Content-derived boxes (chromakey/hue_key) — excluded from this round
  entirely, per `PARKED_WORK.md`.

## Status

Branch `claude/bbox-phase-2` is pushed to origin. Per the agreed cycle,
no PR has been opened yet — awaiting the evaluator's report on this
phase before proceeding to Phase 3 (the consume phase, migrated one
operation at a time).
