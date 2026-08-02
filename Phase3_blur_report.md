# Implementation Report: Phase 3 — BLUR consumes bboxes (first operation)

**Branch:** `claude/bbox-phase-3-blur` (pushed, PR not yet opened — awaiting evaluator re-review)
**Commit:** `38e7960` (fixes `2f28677`'s evaluator-caught blocking bug)
**Spec:** Bounding-box awareness spec, Phase 3 section — per `BBOX_CONVENTIONS.md`

## Summary

Lands the shared Phase 3 infrastructure (`compute_within_bbox`, the
pixels-computed instrumentation hook) and migrates `BLUR` as the first
of eight operations. **This revision fixes one blocking bug the
evaluator caught in the previous round** (see below).

## Fix applied (blocking, from evaluator review of commit `2f28677`)

**Bug:** `execute()`'s masked path intersected `MASK`'s own reported box
against `SOURCE`'s **raw** reported box:
```rust
let natural_box = find_bbox(&ctx.input_bboxes, Input::Source)
    .unwrap_or_else(|| Rect::full(source.width, source.height));
let work_area = natural_box.intersect(&mask_box);
```
But `BLUR`'s true content-spread extent is `SOURCE`'s box **grown by
`radius_px`** — the exact same growth `output_bbox()` (Phase 2) already
computes, since a box blur pulls real neighboring content up to
`radius_px` pixels beyond `SOURCE`'s own non-default region. Using the
un-grown box meant `work_area` could be strictly smaller than what
`BLUR` actually needed to compute, silently skipping real blur
computation in the radius-wide "penumbra" annulus just outside
`SOURCE`'s box whenever `SOURCE` itself reported a sub-frame box (e.g.
fed from a `RESIZE`/`MOVE` chain) — exactly the scenario Phase 1/2 exist
to enable.

None of the original three tests caught it because all three always
used `Rect::full` for `Input::Source`'s reported box — where growing a
full-frame box and intersecting it is a no-op, making the missing
growth mathematically invisible.

**Fix:** factored the shared "`SOURCE`'s box grown by radius, clamped to
frame" logic into a new private `natural_bbox()` helper, used by both
`output_bbox()` (the reported metadata) and `execute()`'s masked path
(the actual work area), so the two can never drift apart again:
```rust
fn natural_bbox(&self, ctx: &Context, input_bboxes: &[(Input, Rect)]) -> Rect {
    let source_box = find_bbox(input_bboxes, Input::Source)
        .unwrap_or_else(|| Rect::full(ctx.meta.width, ctx.meta.height));
    if source_box.is_empty() {
        return Rect::empty();
    }
    source_box.grow(self.radius_px as i32).intersect(&Rect::full(ctx.meta.width, ctx.meta.height))
}
```
`output_bbox()` is now a one-line call to it; `execute()`'s masked path
calls `self.natural_bbox(ctx, &ctx.input_bboxes)` instead of
recomputing the (previously wrong) un-grown box inline.

Added `consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box`
— a regression test using the evaluator's exact counter-example (10×1
frame, `SOURCE` real content confined to `[3,7)`, `RADIUS=2`, `MASK`
fully opaque everywhere), asserting both that restricted-vs-unrestricted
output matches and, directly, that the specific penumbra pixel (`x=1`)
shows real blurred content rather than raw transparent.

All prior tests still pass unchanged.

## What was implemented (otherwise unchanged from prior round)

### Shared infrastructure

- **`engine/src/graphics/mask.rs`**: `compute_within_bbox` — only
  invokes its per-pixel `compute` closure inside a (locally re-clamped)
  work area; everywhere else copied from `original`. Records the actual
  pixel count via a `thread_local` counter.
- **`engine/src/profiling.rs`**: `ProfileEntry.pixels_computed:
  Option<u32>`, `Profile`'s `Display` extended to show it when present.
- **`engine/src/compositor/executors/render.rs`**:
  `evaluate_profiled` resets/reads the counter around each node's
  `execute()` call.
- **`engine/src/graphics/mod.rs`**: re-exports `compute_within_bbox`.

### `BLUR` migration

- **`engine/src/operations/transform/blur.rs`**: `blur_single_pixel` —
  the per-pixel blur primitive, mathematically identical to the
  existing two-pass `blur_pixels` (a separable box blur's per-axis
  pixel count never depends on the other axis, so the two-pass average
  always equals the single-pass 2D window average over the same
  window — independently brute-force-verified by the evaluator across
  40 configurations to ~1e-7 floating-point noise).
  `execute()`'s masked path now correctly uses `natural_bbox()` (grown)
  intersected with `MASK`'s own box as the work area.

## Tests

All from the prior round, plus the new regression test:

| Test | Verifies |
|---|---|
| `consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one` | AC1 |
| **`consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box`** | **new — regression for the fixed blocking bug, using the evaluator's exact counter-example** |
| `a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one` | AC2 |
| `checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off` | AC3 |
| 6 `compute_within_bbox` unit tests in `mask.rs` | shared helper correctness |

**Results:**
```
cargo build   → succeeds, only pre-existing warnings (unrelated dead
                code in graphics/geometry.rs)
cargo test    → 254 passed, 0 failed, 0 ignored (253 prior + 1 new
                regression test)
```

## Acceptance criteria checklist (per BLUR)

1. ✅ Consume-equivalence — **now also verified when `SOURCE` itself
   reports a sub-frame box**, closing the gap the evaluator found.
2. ✅ Instrumentation: strictly fewer pixels computed with a smaller
   mask box.
3. ✅ Graph-level integration: `CHECKERBOARD → RESIZE → MOVE` wired into
   `BLUR`'s `MASK` produces pixel-identical output with bbox consumption
   on vs. off.

## Diff scope

`engine/src/graphics/mask.rs`, `engine/src/graphics/mod.rs`,
`engine/src/profiling.rs`, `engine/src/compositor/executors/render.rs`
(shared infrastructure), and `engine/src/operations/transform/blur.rs`
(the migrated operation, across both commits `2f28677` + the fix
`38e7960`). No other file touched.

## Evaluator comments not otherwise addressed

None — the evaluator's single blocking finding is the fix above; no
major/minor/nit findings were raised.

## Status

Branch `claude/bbox-phase-3-blur` is pushed to origin (commit
`38e7960`). Resubmitting for evaluator re-review before opening a PR or
moving to the next operation (`invert.rs`).
