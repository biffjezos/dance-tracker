# Phase 3 Implementation Report: ADD (RFC-002)

Branch: `claude/bbox-phase-3-add` (based on real `origin/dev` tip `4cab051`, post-RFC-001)
File changed: `engine/src/operations/compose/add.rs` (only file in diff)

`ADD` was accidentally dropped from the original nine-operation Phase 3 list during a mid-series reorder ("ghost first, then screen, subtract and lastly hue_key" implicitly dropped it). RFC-002 closes that gap.

## Design

Mirrors `screen.rs`'s already-landed pattern exactly, since `ADD` shares the same `Foreground`/`Background`/`Mask` shape and the same non-zero-preservation property.

`ADD` is not zero-preserving on either input alone: `0 + x = x`, so a pixel where `Foreground` is default but `Background` carries real content is still genuinely non-default output (`adding_black_is_identity` demonstrates this directly). The natural box is therefore the **union** of `Foreground`'s and `Background`'s own reported boxes, intersected with `MASK`'s box — same reasoning as `SCREEN`/`SUBTRACT`, not an intersection or either box alone.

## Changes

- Added `add_single_pixel(a, b, x, y, width) -> [f32; 4]`, identical math to `add_pixels`'s loop body for a single index.
- `execute()`'s masked path (MASK wired) now computes:
  ```rust
  let natural_box = foreground_box.union(&background_box);
  let work_area = natural_box.intersect(&mask_box);
  ```
  and calls `compute_within_bbox(width, height, work_area, &first_image.pixels, |x, y| add_single_pixel(...))`. The unmasked path (`Self::add_pixels(...)`) is byte-for-byte unchanged. `apply_mask` still runs afterward as the correctness net.
- No `output_bbox()` override, same as `SCREEN`/`SUBTRACT` — compose operations only participate in the consume side this round.

## Tests added (4, adapted from `screen.rs`'s own)

1. `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` — real MASK box vs. full-frame, identical output.
2. `consume_equivalence_requires_the_union_not_the_intersection_of_foreground_and_background_boxes` — the load-bearing one: Foreground reports an empty box, Background reports a real `[3,7)` box on a 10-wide frame; restricted vs. full-frame produce identical output, and Background's real content at x=4 is confirmed actually added in (not silently skipped).
3. `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` — instrumentation via `reset_pixels_computed`/`take_pixels_computed`, 1×1 mask box computes exactly 1 pixel vs. 16 for full-frame on a 4×4 image.
4. `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` — real graph (`CHECKERBOARD → RESIZE → MOVE` as MASK) through `RenderExecutor` ("on") vs. direct `execute()` with empty `ctx.input_bboxes` ("off"), pixel-identical.

## Verification

- `cargo test --lib add::` — 10/10 passed (6 pre-existing + 4 new).
- Full `cargo test` — 287 passed, 0 failed (up from the 283-test baseline post-RFC-001; no regressions).
- `git diff origin/dev --stat` — exactly one file, `add.rs`.
- `ADD`'s unmasked path (`Self::add_pixels`) confirmed byte-for-byte unchanged.

## Build-environment note

None needed this round — `dev`'s tip (`4cab051`, post-RFC-001) compiles cleanly on its own, so no pre-breakage-base workaround was required, unlike the `SCREEN`/`SUBTRACT`/`HUE KEY` rounds.
