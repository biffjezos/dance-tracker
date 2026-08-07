# Implementation Report: Phase 3 — SUBTRACT consumes bboxes (seventh operation)

**Branch:** `claude/bbox-phase-3-subtract` (pushed, PR not yet opened — awaiting evaluator per the phased cycle)
**Commit:** `d325498` (on top of the real `origin/dev` tip, `6f4dd74`)
**Spec:** Bounding-box awareness spec, Phase 3 section — per `BBOX_CONVENTIONS.md`

## Build environment note (same as the SCREEN round)

`dev`'s real tip still carries the unrelated in-flight GPU-backend work
that breaks `cargo build` in this sandbox. Same workflow as last round:
fully developed and verified (`cargo build`/`cargo test`, 279 passed) on
a pre-GPU-work base (`0cde7df`, the last good commit before that work
landed) with `ghost.rs` and `screen.rs`'s already-merged content
restored so the verification baseline matched real `dev` exactly minus
the breaking GPU work. Then extracted the `subtract.rs` diff as a patch
and applied it cleanly onto the actual current `dev` tip (`6f4dd74`) —
confirmed via `git diff origin/dev...HEAD --stat` showing exactly one
file changed.

## Summary

`SUBTRACT` is the seventh of eight operations migrated. With a wired
`MASK`, `execute()` restricts the actual subtraction to the intersection
of `MASK`'s own reported box and this operation's own natural box.

## Same shape as SCREEN, not BLUR/CHROMA KEY/SHUFFLE

`SUBTRACT` is not zero-preserving on either input alone:
`subtract(a, 0) = a` (matches the existing `subtracting_black_is_
identity` test), but `subtract(0, b) = -b` — generally non-default
whenever `Background` alone carries real content, even with `Foreground`
entirely default. The natural box is therefore the **union** of
`Foreground`'s and `Background`'s own reported boxes, exactly the same
reasoning `SCREEN` needed last round (though for a different algebraic
reason — `SCREEN`'s asymmetry comes from `1-(1-a)(1-b)`, `SUBTRACT`'s
from plain linear subtraction).

Verified directly with the same test shape that caught `SCREEN`'s case:
`Foreground` reports an **empty** box, `Background` carries the only
real content, confined to `[3,7)`. Using only the intersection or only
`Foreground`'s box would collapse `work_area` to empty and silently skip
computing the real negative difference `Background`'s content should
produce — the restricted result would wrongly show `Foreground`'s raw
(zero) value instead of a genuine negative out-of-gamut result.

## What was implemented

- **`engine/src/operations/compose/subtract.rs`**: new
  `subtract_single_pixel(a, b, x, y, width)` — identical math to
  `subtract_pixels`'s own loop body for one index. `execute()`'s masked
  path computes `natural_box = union(find_bbox(Foreground),
  find_bbox(Background))` (both falling back to full-frame), intersects
  with `find_bbox(Mask)` into `work_area`, then calls
  `compute_within_bbox` with `Foreground`'s raw pixels as the
  pass-through value (matching `apply_mask`'s own "original" argument).
  Without a `MASK`, the original full-frame `subtract_pixels` path is
  used unchanged.

## Tests

All 5 pre-existing tests pass unchanged, plus:

| Test | Verifies |
|---|---|
| `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` | AC1 |
| **`consume_equivalence_requires_the_union_not_the_intersection_of_foreground_and_background_boxes`** | **the load-bearing test — proves the union is required, with a direct check that the real negative difference from Background's content actually appears** |
| `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` | AC2 — 1 pixel computed vs. 16 for a full-frame box |
| `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` | AC3 |

**Results (on the pre-GPU-work verification branch):**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 279 passed, 0 failed, 0 ignored (275 baseline [including
                GHOST and SCREEN] + 4 new)
```

## Acceptance criteria checklist (per SUBTRACT)

1. ✅ Consume-equivalence, including the union-not-intersection case.
2. ✅ Instrumentation: strictly fewer pixels computed with a smaller
   mask box.
3. ✅ Graph-level integration: `CHECKERBOARD → RESIZE → MOVE` wired into
   `SUBTRACT`'s `MASK` produces pixel-identical output with bbox
   consumption on vs. off.

## Diff scope

Confined to exactly `engine/src/operations/compose/subtract.rs`
(confirmed via `git diff origin/dev...HEAD --stat` against the real
current `dev` tip). No other file touched.

## Status

Branch `claude/bbox-phase-3-subtract` is pushed to origin, based on the
real current `dev` tip. Per the agreed cycle (and the requested reorder:
ghost → screen → subtract → hue_key), no PR has been opened yet —
awaiting the evaluator's report on this operation before proceeding to
the final one, `hue_key.rs`.
