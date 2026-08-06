# Implementation Report: Phase 3 — GHOST consumes bboxes (fifth operation)

**Branch:** `claude/bbox-phase-3-ghost` (pushed, PR not yet opened — awaiting evaluator per the phased cycle)
**Commit:** `206da18`
**Spec:** Bounding-box awareness spec, Phase 3 section — per `BBOX_CONVENTIONS.md`

## Summary

`GHOST` is the fifth of eight operations migrated, and structurally the
most different from the previous four. It does **not** fit the
established `apply_mask`-based pattern (`BLUR`/`INVERT`/`SHUFFLE`/`CHROMA
KEY` all compute a "processed" result then blend it toward "original" by
`MASK`'s weight). `GHOST`'s `MASK` instead feeds directly into a cutout
extraction, and the resulting cutout gets spatially translated across
the whole frame — by up to `GHOST_COUNT * DISTANCE * (SPATIAL_X,
SPATIAL_Y)` pixels — before being composited. This required working out
the correct restriction from first principles rather than reusing the
established recipe verbatim.

## Why only the cutout step is restricted

`GHOST`'s pipeline has two distinct phases:
1. **`cutout_pixels`** — genuinely per-pixel (`cutout[i]` depends only
   on `source[i]`/`mask[i]` at the same index, no neighbors), and
   zero-preserving: RGB always copies `SOURCE` unconditionally, and
   alpha is `source_alpha * mask_alpha`, so `cutout([0,0,0,0], mask)` is
   always `[0,0,0,0]` regardless of `mask`. This step is safely
   restrictable the same way `BLUR`/`SHUFFLE`/`CHROMA KEY`'s per-pixel
   work is.
2. **`translate_pixels` + `composite_over`** (the per-ghost loop) —
   genuinely full-frame: a ghost's cutout gets shifted by
   `n * DISTANCE * (SPATIAL_X, SPATIAL_Y)`, so real, non-transparent
   ghost content can land **anywhere** in the frame, regardless of where
   `MASK`'s own reported box says the *cutout's source* was. Restricting
   this loop's own region to `MASK`'s box (or any function of it) would
   be actively wrong — it would silently clip real, visible ghost
   content wherever a ghost's translation carries it outside that box.
   Doing this correctly would require computing the union of every
   ghost's own translated box first (a separate, larger change, not
   attempted this round) — so this loop is left **untouched, still
   full-frame**, same as before this phase.

## What the skip-substitute value had to be, and why it differs from every prior operation

Every operation migrated so far substitutes `SOURCE`'s own raw pixel for
skipped positions (via `compute_within_bbox`'s `original` parameter),
because their final result is `apply_mask`'s blend, which already
reduces to `SOURCE` wherever `MASK`'s weight is zero — so the
"restricted" and "unrestricted" paths converge there regardless of what
gets computed and discarded.

`GHOST` has no `apply_mask` call. Its cutout's own "untouched" state is
**fully transparent** (`[0,0,0,0]`), not an identity copy of `SOURCE`.
I substitute literal `[0,0,0,0]` for skipped cutout positions instead.
This is still safe: a zero-alpha cutout pixel is always visually inert
downstream regardless of its RGB — `composite_over`'s own formula
divides by (and thus discards a foreground's RGB contribution entirely
whenever) its alpha is `0`. So `[0,0,0,0]` and "`SOURCE`'s raw pixel
with alpha zeroed by `MASK`" produce identical results through the rest
of the pipeline — verified directly by the tests below, not just
asserted.

## What was implemented

- **`engine/src/operations/generators/ghost.rs`**:
  - `render()` factored into a new private `render_with_cutout(cutout,
    width, height)` (the unchanged history/translate/composite logic).
    `render()`'s own public signature and behavior are **completely
    unchanged** — it still always computes an unrestricted cutout first,
    exactly as before this phase, so every one of the 19 pre-existing
    tests (many of which call `render()` directly) pass untouched.
  - New `cutout_single_pixel(source, mask, x, y, width)` — identical
    math to `cutout_pixels`'s own loop body for one index.
  - `execute()`: with a wired `MASK`, computes `work_area =
    intersect(find_bbox(Input::Source), find_bbox(Input::Mask))` (both
    falling back to full-frame), calls `compute_within_bbox` with a
    `transparent` (all-zero) buffer as the pass-through value and a
    closure wrapping `cutout_single_pixel`, then passes the result to
    `render_with_cutout` directly (bypassing `render()`'s own
    unrestricted cutout computation). Without a `MASK`, the same
    opaque-mask fallback `render()` used internally is replicated inline
    (small, unavoidable duplication given `render()`'s signature stays
    frozen).

## Tests

All 19 pre-existing tests pass unchanged, plus:

| Test | Verifies |
|---|---|
| `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` | AC1 (using `GHOST_COUNT=0`/`SHOW_SOURCE=true` to isolate the cutout-restriction logic) |
| `consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box` | confirms zero-preservation w.r.t. `SOURCE` directly |
| **`a_ghost_translated_outside_masks_own_box_still_renders_correctly`** | **the load-bearing test for this operation** — a ghost offset well outside `MASK`'s own tight `[0,1)` box still shows real, opaque content at its translated position, proving the cutout-only restriction didn't also (incorrectly) restrict the translate/composite loop |
| `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` | AC2 — 1 pixel computed vs. 16 for a full-frame box |
| `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` | AC3 |

**Results:**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 271 passed, 0 failed, 0 ignored (266 baseline + 5 new)
```

## Acceptance criteria checklist (per GHOST)

1. ✅ Consume-equivalence, including the sub-frame-`SOURCE` case.
2. ✅ Instrumentation: strictly fewer pixels computed with a smaller
   mask box (in the cutout step specifically).
3. ✅ Graph-level integration: `CHECKERBOARD → RESIZE → MOVE` wired into
   `GHOST`'s `MASK` produces pixel-identical output with bbox
   consumption on vs. off.

## Scope note — what this round does *not* optimize

Only the cutout step is restricted. The translate/composite loop —
`GHOST`'s actual dominant cost (`O(width * height)` per ghost, run
`ghost_count` times, and `GHOST` is always `is_live()`, never
cross-tick-cached) — remains full-frame. A deeper optimization that also
restricts that loop would need to compute the union of every ghost's own
translated box (`MASK`'s box shifted by `n * DISTANCE * (SPATIAL_X,
SPATIAL_Y)` for `n = 1..=GHOST_COUNT`, unioned together, plus `SOURCE`'s
own box if `SHOW_SOURCE`) before restricting the composite step to that
region — a correct, larger follow-up, not attempted here given the
risk/complexity relative to the other seven (simpler) operations in this
round.

## Diff scope

Confined to exactly `engine/src/operations/generators/ghost.rs`
(confirmed via `git status --short`). No other file touched.

## Status

Branch `claude/bbox-phase-3-ghost` is pushed to origin. Per the agreed
cycle (and the requested reorder: ghost → screen → subtract → hue_key),
no PR has been opened yet — awaiting the evaluator's report on this
operation before proceeding to `screen.rs`.
