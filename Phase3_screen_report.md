# Implementation Report: Phase 3 — SCREEN consumes bboxes (sixth operation)

**Branch:** `claude/bbox-phase-3-screen` (pushed, PR not yet opened — awaiting evaluator per the phased cycle)
**Commit:** `3611a45` (on top of the real `origin/dev` tip, `2d2e6f0`)
**Spec:** Bounding-box awareness spec, Phase 3 section — per `BBOX_CONVENTIONS.md`

## A build environment note (not part of the SCREEN change itself)

`dev`'s actual current tip (`2d2e6f0`) has unrelated, in-flight GPU-backend
work (`wgpu`/`pollster`/`bytemuck` dependencies, a `compute` module) that
currently **breaks the build in this sandbox**: `wgpu`'s dependency tree
can't be fetched (network policy blocks `static.crates.io` in this
environment), and separately, `Context` lost its `#[derive(Default)]`
and gained a required `compute: Arc<dyn ComputeBackend>` field with no
default — which would break `Context::default()`/`..Default::default()`
construction used throughout every operation's existing tests, this
change's own new tests included.

Per your direction, I developed and fully verified this change
(`cargo build`/`cargo test`, 275 passed) on a branch based on `0cde7df`
(the last commit before that work landed — one before the `GHOST` merge),
with `ghost.rs`'s own already-merged content manually restored so the
verification baseline matched real `dev` exactly except for the breaking
GPU work. Once verified, I extracted the `screen.rs` diff as a patch and
applied it cleanly onto the actual current `dev` tip (`2d2e6f0`) for this
branch/PR, so the PR itself is based on real `dev` and contains only the
`screen.rs` change — confirmed via `git diff origin/dev...HEAD --stat`
showing exactly one file. I was not able to re-run `cargo test` against
that exact final commit due to the same network/build blocker, but the
`screen.rs` diff applied byte-for-byte identically to the version that
did pass the full suite, so I'm confident it's correct — flagging this
so it isn't taken as full CI-equivalent verification.

## Summary

`SCREEN` is the sixth of eight operations migrated. With a wired `MASK`,
`execute()` restricts the actual screen computation to the intersection
of `MASK`'s own reported box and this operation's own natural box.

## Why the natural box is a union here, not one input's box

Every operation migrated so far (except `GHOST`, which doesn't use this
shape at all) used **one input's own box** as the natural-box operand,
because each was zero-preserving on that one input. `SCREEN` breaks that
pattern differently than `INVERT` did: it's not zero-preserving on
*either* input **alone** — `screen(0, b) = b` (the existing
`screening_with_black_is_identity` test proves this directly: screening
black with a real colour reproduces that colour unchanged). So a pixel
where `Foreground` is entirely default but `Background` carries real
content is still genuinely non-default output — the reverse is true too.
The correct natural box is therefore the **union** of `Foreground`'s and
`Background`'s own reported boxes: the region where *either* input could
contribute real content, not the intersection (which would be too
aggressive) and not just one of them (which would silently drop the
other's real content).

I verified this directly with a regression test (`consume_equivalence_
requires_the_union_not_the_intersection_of_foreground_and_background_
boxes`) using an **empty** `Foreground` box and a real `Background` box
confined to `[3,7)`: using only the intersection or only `Foreground`'s
box would collapse `work_area` to empty and silently skip screening in
`Background`'s real content, wrongly leaving black. The test pins down
both the overall consume-equivalence and the specific pixel that would
go wrong.

## What was implemented

- **`engine/src/operations/compose/screen.rs`**: new
  `screen_single_pixel(a, b, x, y, width)` — identical math to
  `screen_pixels`'s own loop body for one index. `execute()`'s masked
  path computes `natural_box = union(find_bbox(Foreground),
  find_bbox(Background))` (both falling back to full-frame), intersects
  with `find_bbox(Mask)` into `work_area`, then calls
  `compute_within_bbox` with `Foreground`'s raw pixels as the
  pass-through value (matching `apply_mask`'s own "original" argument —
  same convention as every operation except `GHOST`). Without a `MASK`,
  the original full-frame `screen_pixels` path is used unchanged.

## Tests

All 5 pre-existing tests pass unchanged (verified pre-patch), plus:

| Test | Verifies |
|---|---|
| `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` | AC1 |
| **`consume_equivalence_requires_the_union_not_the_intersection_of_foreground_and_background_boxes`** | **the load-bearing test — proves the union (not intersection, not one input alone) is required, with a direct pixel-level check that Background's real content is actually screened in** |
| `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` | AC2 — 1 pixel computed vs. 16 for a full-frame box |
| `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` | AC3 |

**Results (on the pre-GPU-work verification branch):**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 275 passed, 0 failed, 0 ignored (271 baseline [including
                GHOST] + 4 new)
```

## Acceptance criteria checklist (per SCREEN)

1. ✅ Consume-equivalence, including the union-not-intersection case.
2. ✅ Instrumentation: strictly fewer pixels computed with a smaller
   mask box.
3. ✅ Graph-level integration: `CHECKERBOARD → RESIZE → MOVE` wired into
   `SCREEN`'s `MASK` produces pixel-identical output with bbox
   consumption on vs. off.

## Diff scope

Confined to exactly `engine/src/operations/compose/screen.rs` (confirmed
via `git diff origin/dev...HEAD --stat` against the real current `dev`
tip). No other file touched.

## Status

Branch `claude/bbox-phase-3-screen` is pushed to origin, based on the
real current `dev` tip. Per the agreed cycle (and the requested reorder:
ghost → screen → subtract → hue_key), no PR has been opened yet —
awaiting the evaluator's report on this operation before proceeding to
`subtract.rs`.
