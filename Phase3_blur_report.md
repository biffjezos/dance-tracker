# Implementation Report: Phase 3 — BLUR consumes bboxes (first operation)

**Branch:** `claude/bbox-phase-3-blur` (pushed, PR not yet opened — awaiting evaluator per the phased cycle)
**Commit:** `2f28677`
**Spec:** Bounding-box awareness spec, Phase 3 section — per `BBOX_CONVENTIONS.md`

## Summary

Lands the shared Phase 3 infrastructure (`compute_within_bbox`, the
pixels-computed instrumentation hook) and migrates the first of eight
operations named in the spec: `BLUR`. With a wired `MASK`, `execute()`
now restricts the actual blur computation to the intersection of
`MASK`'s own reported box and `SOURCE`'s own natural box, instead of
running the full two-pass blur over the whole frame unconditionally.
Per the spec's explicit instruction, this lands as its own
independently-mergeable unit — the other seven operations
(`invert.rs`, `shuffle.rs`, `chromakey.rs`, `hue_key.rs`, `ghost.rs`,
`add.rs`, `screen.rs`, `subtract.rs`) are follow-ups, not bundled here.

## What was implemented

### Shared infrastructure (new, reused by every future Phase 3 operation)

- **`engine/src/graphics/mask.rs`**: `compute_within_bbox(width, height,
  work_area, original, compute)` — only invokes the `compute` closure
  for pixels inside `work_area` (clamped to `[0,width) x [0,height)`
  regardless of whether the caller already clamped it); everywhere else
  is copied directly from `original`. Records how many pixels were
  actually computed via a `thread_local` counter
  (`LAST_PIXELS_COMPUTED`), read via `take_pixels_computed()` and reset
  via `reset_pixels_computed()`.
- **`engine/src/profiling.rs`**: `ProfileEntry` gains `pixels_computed:
  Option<u32>` — `None` for any node that never calls
  `compute_within_bbox` (no wired `MASK`, or not yet migrated), `Some(n)`
  otherwise. `Profile`'s `Display` impl includes the count when present.
  This extends the existing instrumentation mechanism rather than
  inventing a separate one, per the spec's explicit instruction.
- **`engine/src/compositor/executors/render.rs`**:
  `RenderExecutor::evaluate_profiled` resets the counter immediately
  before each node's `execute()` call and reads it immediately after,
  so a node that doesn't call `compute_within_bbox` at all correctly
  reports `None` rather than a stale count left over from an earlier
  node in the same tick.
- **`engine/src/graphics/mod.rs`**: re-exports `compute_within_bbox`.

### `BLUR` migration

- **`engine/src/operations/transform/blur.rs`**: new
  `Blur::blur_single_pixel(pixels, width, height, radius, x, y)` — the
  blurred value of one output pixel, computed directly from raw source
  pixels via the same per-axis clipped window `blur_pixels` uses.
  **This is mathematically identical to the existing two-pass
  `blur_pixels`, not an approximation**: a separable box blur's per-axis
  pixel count at a given output `x` never depends on `y` (and vice
  versa), so the two-pass average
  `(1/countY) * sum_y[ (1/countX) * sum_x pixels ]` always equals the
  single-pass 2D window average `sum / (countX * countY)` over the same
  rectangular window — the two passes' independent normalizations
  factor apart exactly.
  - `execute()`: when `MASK` is wired, intersects `find_bbox(Input::Source)`
    (falling back to full-frame) with `find_bbox(Input::Mask)` (same
    fallback) into `work_area`, then calls `compute_within_bbox` with a
    closure wrapping `blur_single_pixel`. Without a wired `MASK`, the
    original full-frame `blur_pixels` path is used unchanged — there's
    nothing to restrict against.
  - The final `apply_mask` blend step is unchanged in both paths.

## Tests

All existing `blur.rs` tests pass unchanged (14), including the two
that already exercised the masked path
(`a_zero_alpha_mask_suppresses_the_blur_entirely`,
`a_full_alpha_mask_applies_the_blur_exactly_as_unmasked`) — both still
pass against the new restricted-compute code path, which is itself
strong evidence `blur_single_pixel` reproduces `blur_pixels` exactly.

New tests, per the spec's per-operation acceptance criteria:

| Test | Verifies |
|---|---|
| `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` | **AC1** — same `Source`/`Mask` pixel data, once with a real (tight) reported `Mask` box, once with the fallback full-frame box; identical output |
| `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` | **AC2** — a 1×1 mask box records exactly 1 computed pixel vs. 16 for a full-frame 4×4 box, read via the new `pixels_computed` instrumentation |
| `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` | **AC3** — `CHECKERBOARD → RESIZE → MOVE` wired as `BLUR`'s own `MASK`, through a real `Graph`/`RenderExecutor` ("on" — real boxes threaded) compared against a direct `BLUR::execute()` call with the same resolved input `Value`s but empty `ctx.input_bboxes` ("off" — simulating pre-Phase-3 always-full-frame behavior); identical output |

Plus 6 new unit tests for `compute_within_bbox` itself in `mask.rs`
(work-area restriction, exact per-pixel call count, empty work area
never calls `compute`, out-of-bounds work area gets clamped, the
pixel-count instrumentation hook records/clears correctly).

**Results:**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 253 passed, 0 failed, 0 ignored (250 baseline + 3 new
                in blur.rs — the 6 mask.rs tests are folded into that
                250 baseline count from the infra commit)
```

## Acceptance criteria checklist (per BLUR)

1. ✅ Consume-equivalence: identical output with a real (smaller) `Mask`
   bbox vs. a full-frame one.
2. ✅ Instrumentation: strictly fewer pixels computed with a smaller
   mask box (1 vs. 16 in the test), read via the extended
   `Profile`/`ProfileEntry`.
3. ✅ Graph-level integration: a geometric mask
   (`CHECKERBOARD → RESIZE → MOVE`) wired into `BLUR`'s `MASK` produces
   pixel-identical output with bbox consumption on vs. off.

## Diff scope

`engine/src/graphics/mask.rs`, `engine/src/graphics/mod.rs`,
`engine/src/profiling.rs`, `engine/src/compositor/executors/render.rs`
(the shared infrastructure — necessarily bundled with the first
consumer), and `engine/src/operations/transform/blur.rs` (the migrated
operation). No other operation touched.

## Out of scope this phase (per spec, untouched)

- The remaining seven operations (`invert.rs`, `shuffle.rs`,
  `chromakey.rs`, `hue_key.rs`, `ghost.rs`, `add.rs`, `screen.rs`,
  `subtract.rs`) — each is its own follow-up commit/PR, per the spec's
  explicit "don't batch all eight into one change."
- Content-derived boxes (chromakey/hue_key's own keyed-out alpha) —
  excluded from this round entirely, per `PARKED_WORK.md`. Only
  chromakey/hue_key's own wired `Input::Mask` will be in scope when
  their turn comes.

## Status

Branch `claude/bbox-phase-3-blur` is pushed to origin. Per the agreed
cycle, no PR has been opened yet — awaiting the evaluator's report on
this operation before proceeding to the next one (`invert.rs`).
