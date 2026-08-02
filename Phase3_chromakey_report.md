# Implementation Report: Phase 3 — CHROMA KEY consumes bboxes (fourth operation)

**Branch:** `claude/bbox-phase-3-chromakey` (pushed, PR not yet opened — awaiting evaluator per the phased cycle)
**Commit:** `657db5d`
**Spec:** Bounding-box awareness spec, Phase 3 section — per `BBOX_CONVENTIONS.md`

## Summary

`CHROMA KEY` is the fourth of eight operations migrated to consume
boxes. With a wired `MASK`, `execute()` restricts the actual keying
computation to the intersection of `MASK`'s own reported box and
`SOURCE`'s own reported box — no growth needed, since `key_pixels` reads
only the pixel it writes (no neighbors). Only its own wired `MASK` is in
scope, per the spec — deriving a box from its own keyed-out alpha
(content-derived tightening) is the excluded Phase 4.

## Zero-preservation check (per the discipline established since `INVERT`)

`key_pixels`'s loop body always copies RGB through unchanged
(`target[0..3] = source[0..3]`), and sets alpha to either `0.0`
(explicitly keyed) or `source[3]` (untouched) — never anything else. For
a source pixel `[0,0,0,0]`: RGB stays `[0,0,0]`, and alpha is either
explicitly `0.0` or the already-`0.0` `source[3]` — either way, the
output is `[0,0,0,0]`, **for any `KEY_COLOR`/`THRESHOLD`**. `CHROMA KEY`
is zero-preserving, so `SOURCE`'s own box is a valid intersection
operand, verified directly with a sub-frame-`SOURCE` regression test
(mixing a pure-green pixel that keys out and other colours that don't,
inside the restricted region itself) — passed on the first attempt.

## What was implemented

- **`engine/src/operations/key/chromakey.rs`**: new
  `key_single_pixel(pixels, key_color, threshold, x, y, width)` — the
  keyed value of one pixel, identical math to `key_pixels`'s own loop
  body for that index. `execute()`'s masked path computes `work_area =
  intersect(find_bbox(Input::Source), find_bbox(Input::Mask))` (both
  falling back to full-frame), then calls `compute_within_bbox` with a
  closure wrapping `key_single_pixel`. Without a `MASK`, the original
  full-frame `key_pixels` path is used unchanged. No `output_bbox()`
  override — `CHROMA KEY`'s natural box already equals `SOURCE`'s own
  box, nothing further to tighten (and reporting a box from its own
  keyed-out alpha is explicitly out of scope this round).

## Tests

All 11 pre-existing tests pass unchanged, plus:

| Test | Verifies |
|---|---|
| `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` | AC1 |
| `consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box` | confirms zero-preservation directly, using a mix of keyed and non-keyed colours inside the restricted region |
| `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` | AC2 — 1 pixel computed vs. 16 for a full-frame box |
| `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` | AC3 |

**Results:**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 266 passed, 0 failed, 0 ignored (262 baseline + 4 new)
```

## Acceptance criteria checklist (per CHROMA KEY)

1. ✅ Consume-equivalence, including the sub-frame-`SOURCE` case.
2. ✅ Instrumentation: strictly fewer pixels computed with a smaller
   mask box.
3. ✅ Graph-level integration: `CHECKERBOARD → RESIZE → MOVE` wired into
   `CHROMA KEY`'s `MASK` produces pixel-identical output with bbox
   consumption on vs. off.

## Diff scope

Confined to exactly `engine/src/operations/key/chromakey.rs`
(confirmed via `git status --short`). No other file touched.

## Out of scope (per spec, untouched)

Deriving a box from `CHROMA KEY`'s own keyed-out alpha (content-derived
tightening) — excluded from this round entirely, per `PARKED_WORK.md`'s
"Content-derived bbox tightening for chromakey/hue_key" entry.

## Status

Branch `claude/bbox-phase-3-chromakey` is pushed to origin. Per the
agreed cycle, no PR has been opened yet — awaiting the evaluator's
report on this operation before proceeding to the next one
(`hue_key.rs`).
