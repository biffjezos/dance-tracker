---
---
# Implementation Report: Phase 3 — SHUFFLE consumes bboxes (third operation)

**Branch:** `claude/bbox-phase-3-shuffle` (pushed, PR not yet opened — awaiting evaluator per the phased cycle)
**Commit:** `ad38bf4`
**Spec:** Bounding-box awareness spec, Phase 3 section — per `BBOX_CONVENTIONS.md`

## Summary

`SHUFFLE` is the third of eight operations migrated to consume boxes.
With a wired `MASK`, `execute()` restricts the actual shuffle
computation to the intersection of `MASK`'s own reported box and
`SOURCE`'s own reported box — no growth needed, since `SHUFFLE` reads
only the same pixel it writes (no neighbors, unlike `BLUR`'s window).

## Zero-preservation check (per the lesson from `INVERT`)

Given `INVERT` turned out not to be zero-preserving, I checked
`SHUFFLE`'s own property directly rather than assuming `BLUR`'s
"intersect with `SOURCE`'s box" pattern transfers: `shuffle_pixels`'s
per-channel output is always either a copy of one of the source's own 4
channels, or `Off`'s `T::default()` (`0`). Every possible channel
mapping therefore maps `[0,0,0,0]` to `[0,0,0,0]` — `SHUFFLE` **is**
zero-preserving, unlike `INVERT`. So `SOURCE`'s own reported box (where
its content is genuinely non-default) correctly bounds where
`SHUFFLE`'s own unmasked output can be non-default too, with no growth
needed (no neighbor pixels are ever read). This means `SOURCE`'s raw
box **is** a valid intersection operand here, verified directly with a
sub-frame-`SOURCE` regression test (the same test shape that caught
`INVERT`'s bug) rather than assumed — it passed on the first attempt.

No `output_bbox()` override was added or needed — `SHUFFLE`'s natural
box genuinely is `SOURCE`'s own box (once one is reported upstream);
since nothing in this round adds `output_bbox()` reporting to
`SHUFFLE` itself, downstream nodes still see the inherited full-frame
default from it, same as before.

## What was implemented

- **`engine/src/operations/transform/shuffle.rs`**: `execute()`'s masked
  path computes `work_area = intersect(find_bbox(Input::Source),
  find_bbox(Input::Mask))` (both falling back to full-frame), then calls
  `compute_within_bbox` with a closure that reads the one source pixel at
  `(x,y)` and applies the existing `channel_value` per-channel mapping
  directly — the same logic `shuffle_pixels` uses, just per-pixel instead
  of over the whole buffer. Without a `MASK`, the original full-frame
  `shuffle_pixels` path is used unchanged.

## Tests

All 7 pre-existing tests pass unchanged, plus:

| Test | Verifies |
|---|---|
| `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` | AC1 |
| `consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box` | confirms the zero-preservation reasoning above directly, not just by inference |
| `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` | AC2 — 1 pixel computed vs. 16 for a full-frame box |
| `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` | AC3 |

**Results:**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 262 passed, 0 failed, 0 ignored (258 baseline + 4 new)
```

## Acceptance criteria checklist (per SHUFFLE)

1. ✅ Consume-equivalence, including the sub-frame-`SOURCE` case.
2. ✅ Instrumentation: strictly fewer pixels computed with a smaller
   mask box.
3. ✅ Graph-level integration: `CHECKERBOARD → RESIZE → MOVE` wired into
   `SHUFFLE`'s `MASK` produces pixel-identical output with bbox
   consumption on vs. off.

## Diff scope

Confined to exactly `engine/src/operations/transform/shuffle.rs`
(confirmed via `git status --short`). No other file touched.

## Status

Branch `claude/bbox-phase-3-shuffle` is pushed to origin. Per the
agreed cycle, no PR has been opened yet — awaiting the evaluator's
report on this operation before proceeding to the next one
(`chromakey.rs`).
