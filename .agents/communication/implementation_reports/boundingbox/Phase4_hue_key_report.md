# Phase 3 Implementation Report: HUE KEY

Branch: `claude/bbox-phase-3-huekey-real` (based on real `origin/dev` tip `689d0d5`)
File changed: `engine/src/operations/key/hue_key.rs` (only file in diff)

This is the eighth and final operation of Phase 3 (BBOX_CONVENTIONS.md).

## Zero-preservation analysis

`key_pixels`' per-pixel body:

```rust
let reference_hue = reference[idx] as f64 * 360.0;
let distance = Self::hue_distance(reference_hue, target_hue);
[
    source[idx], source[idx + 1], source[idx + 2],
    if distance <= threshold { 0.0 } else { source[idx + 3] },
]
```

RGB is always copied from SOURCE unconditionally. Alpha is either explicitly
zeroed (keyed out) or left at SOURCE's own existing alpha. Whichever branch is
taken, if SOURCE's pixel is already `[0,0,0,0]`, the output is `[0,0,0,0]`
regardless of REFERENCE's value. This is structurally identical to CHROMA
KEY's zero-preservation property (same "always copy SOURCE's own alpha/RGB,
branch decides zero-vs-passthrough" shape).

REFERENCE's own reported box is irrelevant to the restriction: it only
decides *which* branch alpha takes, never whether the result is zero when
SOURCE already is. REFERENCE's full pixel buffer is read unrestricted
wherever `key_single_pixel` runs, regardless of `work_area`.

No growth is needed: `key_single_pixel` reads only the pixel at `(x, y)`,
no neighbors.

## Changes

- Added `key_single_pixel(source, reference, target_hue, threshold, x, y, width) -> [f32; 4]`, identical math to `key_pixels`'s loop body for a single index.
- `execute()`'s masked path (MASK wired) now computes:
  ```rust
  let natural_box = find_bbox(&ctx.input_bboxes, Input::Source).unwrap_or_else(|| Rect::full(w, h));
  let mask_box = find_bbox(&ctx.input_bboxes, Input::Mask).unwrap_or_else(|| Rect::full(w, h));
  let work_area = natural_box.intersect(&mask_box);
  ```
  and calls `compute_within_bbox(width, height, work_area, &source_pixels, |x, y| key_single_pixel(...))`, with SOURCE's raw pixels as pass-through outside `work_area`. The unmasked path is unchanged (`key_pixels` over the full frame). `apply_mask` still runs afterward as the correctness net.

## Tests added (4)

1. `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` — real MASK box vs. full-frame, identical output.
2. `consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box` — 10-wide frame, SOURCE content confined to `[3,7)` mixing a keyed-out and a kept pixel, SOURCE reports box `[3,7)`; restricted vs. full-frame produce identical output.
3. `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` — instrumentation via `reset_pixels_computed`/`take_pixels_computed`, 1×1 mask box computes exactly 1 pixel vs. 16 for full-frame on a 4×4 image.
4. `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` — real graph (`CHECKERBOARD → RESIZE → MOVE` as MASK) through `RenderExecutor` ("on") vs. direct `execute()` with empty `ctx.input_bboxes` ("off"), pixel-identical.

## Verification

- `cargo test --lib hue_key::` — 11/11 passed (7 pre-existing + 4 new).
- Full `cargo test` — 283 passed, 0 failed (up from the 279-test baseline; no regressions).

## Build-environment note (same caveat as SCREEN/SUBTRACT reports)

`dev`'s real tip added an unconditional `wgpu = "30.0.0"` dependency (plus
`pollster`/`bytemuck`) that requires fetching from `static.crates.io`, which
is policy-blocked in this sandbox. All implementation and verification
(`cargo build`/`cargo test`) was done on a branch based on the last clean
pre-`wgpu` commit (`0cde7df`), with the already-merged GHOST/SCREEN/SUBTRACT
files restored from the real `dev` tip so the local tree matches it exactly
except for the unfetchable dependency bump. The final commit was produced by
diffing just `hue_key.rs` and applying that patch cleanly onto the real
`origin/dev` tip (`689d0d5`), verified via `git diff origin/dev --stat`
showing only `hue_key.rs` changed. This mirrors the SCREEN/SUBTRACT workflow,
which the evaluator independently reproduced and confirmed compiles/passes
on a build with working `wgpu` access.
