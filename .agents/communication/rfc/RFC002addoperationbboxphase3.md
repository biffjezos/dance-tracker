# RFC-002: Add the missing `ADD` operation to the bounding-box Phase 3 rollout

**Status:** Ready once RFC-001 lands (needs a compiling tree).
**Type:** RFC, not a new Specification — this completes work already scoped by the original bounding-box implementation spec and `BBOX_CONVENTIONS.md`; it doesn't design anything new.

## Background

The original bounding-box implementation spec's Phase 3 listed nine operations to migrate: `blur`, `invert`, `shuffle` (transform); `chromakey`, `hue_key` (key); `ghost` (generators); `add`, `screen`, `subtract` (compose). Eight landed, each with its own implementation report and passing tests. **`add.rs` was never migrated** — `git grep` for `find_bbox`/`compute_within_bbox` in `engine/src/operations/compose/add.rs` returns nothing, and no branch/PR for it exists. This RFC closes that gap.

`ADD` is architecturally identical in shape to `SCREEN` and `SUBTRACT` (both already migrated) — same `Foreground`/`Background`/`Mask` inputs, same "combine two full-frame inputs" category. Follow `screen.rs`'s already-landed pattern exactly; this is not a design decision, it's applying the established one.

## Design (mirrors `screen.rs`)

In `ADD::execute()`, when a `MASK` is wired, restrict the actual per-pixel add to the region that can matter, instead of computing the whole frame unconditionally:

```rust
let foreground_box = find_bbox(&ctx.input_bboxes, Input::Foreground)
    .unwrap_or_else(|| Rect::full(first_image.width, first_image.height));
let background_box = find_bbox(&ctx.input_bboxes, Input::Background)
    .unwrap_or_else(|| Rect::full(first_image.width, first_image.height));
let natural_box = foreground_box.union(&background_box);
let mask_box = find_bbox(&ctx.input_bboxes, Input::Mask)
    .unwrap_or_else(|| Rect::full(first_image.width, first_image.height));
let work_area = natural_box.intersect(&mask_box);

let width = first_image.width;
let a = &first_image.pixels;
let b = &second_image.pixels;

let added = crate::graphics::compute_within_bbox(width, first_image.height, work_area, a, |x, y| {
    Self::add_single_pixel(a, b, x, y, width)
});
```

**The union, not the intersection, is load-bearing here — same as `SCREEN`.** `ADD`'s identity is `0 + x = x`: if `Foreground` is entirely default/transparent and `Background` carries the only real content, the natural box has to be the *union* of both inputs' reported boxes, not just one or their intersection — otherwise real `Background` content gets silently skipped whenever `Foreground`'s own box happens to be smaller or empty. Add a new private `add_single_pixel(a, b, x, y, width) -> [f32; 4]` next to the existing `add_pixels` (same relationship `screen_single_pixel`/`screen_pixels` already has in `screen.rs`) — same math as `add_pixels`, just addressed by `(x, y)` instead of a whole-buffer loop, so `compute_within_bbox` can call it per-pixel.

Without a wired `MASK`, keep the existing unconditional `Self::add_pixels(...)` call unchanged — there's nothing to restrict against.

No `output_bbox()` override — same as `screen.rs`/`subtract.rs`, compose operations only participate in the consume side this round, not the report side (they have no single "natural size" the way a transform like `BLUR`/`RESIZE` does).

## Tests

Add these four, adapted from `screen.rs`'s own (mirror the test bodies exactly, swapping `Screen`/`screen` for `Add`/`add` and the actual math):

- `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one`
- `consume_equivalence_requires_the_union_not_the_intersection_of_foreground_and_background_boxes` — the load-bearing one; `screen.rs`'s version (lines ~337-390) is the exact template, including its own explanatory comment, which transfers near-verbatim to `ADD` (arguably even more directly obvious for `ADD` than `SCREEN`, since `0 + x = x` is a simpler identity than screen's).
- `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one`
- `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off`

Write an implementation report (`Phase3_add_report.md`, matching the format of the existing `Phase0_bbox_report.md` through `Phase3_blur_report.md`/`Phase3_*` reports) documenting what changed and the test results, same as every prior phase.

## Acceptance criteria

1. `cargo build`/`cargo test` succeed, all four new tests pass alongside the full existing suite.
2. `ADD`'s unmasked path (`Self::add_pixels`) is byte-for-byte unchanged.
3. `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` proves the same end-to-end garbage-matte scenario used for every other Phase 3 operation also holds for `ADD`.
4. No other file touched besides `engine/src/operations/compose/add.rs` and the new implementation report.
