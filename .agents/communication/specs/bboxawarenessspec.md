# Spec: Bounding-box awareness (this round: crop/resize/move/blur/geometric masks)

## Inputs to this spec

Read `BBOX_CONVENTIONS.md` (repo root) first - it's the architecture
decisions this spec implements against: the `Rect` type, the two
correctness invariants, why `Context` gets a new `input_bboxes` field
instead of `execute()` gaining a parameter, and the report-vs-consume
split. This document does not repeat that reasoning - it's the phased,
file-level build plan on top of it.

**Scope, per management's decision:** `RESIZE`, `MOVE` (once built per its
own spec), `BLUR`, and geometric (non-content-derived) masks - e.g. a
rectangular garbage matte built from `CHECKERBOARD` + `RESIZE` + `MOVE`.
Chromakey/hue_key content-derived tightening is explicitly **out of
scope** for this round - see `PARKED_WORK.md`'s "Content-derived bbox
tightening for chromakey/hue_key" entry. Do not attempt it as part of this
work; the trait signature already leaves room for it later (see below), so
there's nothing to design around now.

## Phase 0 — Foundation (all categories, zero behavior change)

**Files:**
- New `engine/src/compositor/bbox.rs`: the `Rect` type (`full`, `empty`,
  `is_empty`, `intersect`, `union`, `grow`) per `BBOX_CONVENTIONS.md`.
- `engine/src/compositor/input.rs`: add `find_bbox(bboxes: &[(Input,
  Rect)], key: Input) -> Option<Rect>` next to the existing `find_input`,
  same shape.
- `engine/src/compositor/operations.rs`: add `Operation::output_bbox`
  with the full-frame default, exact signature from
  `BBOX_CONVENTIONS.md`:
  ```rust
  fn output_bbox(&self, ctx: &Context, input_bboxes: &[(Input, Rect)], output: &Value) -> Rect {
      Rect::full(ctx.meta.width, ctx.meta.height)
  }
  ```
- `engine/src/compositor/context.rs`: add `pub input_bboxes: Vec<(Input,
  Rect)>` to `Context` (already `#[derive(Default)]`s - no other
  construction site needs to change).
- `engine/src/compositor/executors/render.rs`: `RenderExecutor::evaluate`
  and `evaluate_profiled` thread `(Value, Rect)` instead of bare `Value`
  through the recursion; `CachedNode` gains a `bbox: Rect` field. Build
  `input_bboxes` from resolved input `Rect`s, construct a per-node
  `Context` (clone + override `input_bboxes`) immediately before calling
  `execute()`, then call `output_bbox(ctx, &input_bboxes, &value)` for
  this node's own box.
- `engine/src/compositor/executors/preview.rs`: same threading for
  `PreviewExecutor::evaluate_memoized` and `evaluate_unmemoized`.
- **No change** to the public `Execute::execute()` signature
  (`executors/mod.rs`) - still returns `Vec<Value>`. This threading is
  private to each executor's own recursion.
- **No change** to any individual operation file. Every operation
  inherits the full-frame default with zero edits.

**Acceptance criteria:**
1. `cargo build`/`cargo test` pass with zero test modifications anywhere
   outside `compositor/` - if any existing operation test needed editing
   to compile, something in this phase touched more than it should have.
2. New unit test: an arbitrary operation's `output_bbox()` (using the
   default, unoverridden) returns exactly `Rect::full(ctx.meta.width,
   ctx.meta.height)`.
3. New unit tests for `Rect::intersect`/`union`/`grow`, including the
   empty-operand edge cases called out in `BBOX_CONVENTIONS.md` (`union`
   with an empty operand returns the other operand unchanged, not
   something extended to include the origin).
4. A render/preview integration test confirms output is byte-identical
   to a pre-Phase-0 run for at least one existing multi-node graph (e.g.
   a `chromakey` → `add` chain) - proves the new threading changed
   nothing observable yet.

## Phase 1 — Geometric producers report (RESIZE, MOVE)

**Files:** `engine/src/operations/transform/resize.rs`,
`engine/src/operations/transform/move.rs` (once it exists - if MOVE
hasn't landed yet when this phase starts, do `RESIZE` alone and leave
`MOVE` as a follow-up to this phase, not a blocker for it).

Both override `output_bbox()` to remap their `Source` input's box through
their own parameters - pure arithmetic, no pixel reads, and (important)
**neither operation's `execute()` changes in this phase** - this is
report-only. `RESIZE` at `scale < 100` shrinks the box toward center by
the inverse of `resize_pixels`'s own center-relative mapping; `MOVE`
translates the box by `(OFFSET_X, OFFSET_Y)`, clamped to the frame.

**Acceptance criteria:**
1. `RESIZE` at `50%` on a full-frame input reports a box exactly half the
   frame's width/height, centered.
2. `MOVE` with a nonzero offset on a full-frame input reports a box
   translated by exactly that offset, clamped to the frame bounds (an
   offset larger than the frame reports `Rect::empty()`, not a box with
   negative extent).
3. Chaining `RESIZE`/`MOVE` into any *unmodified* downstream operation
   (e.g. `INVERT`, which hasn't been touched) still produces pixel-
   identical output to today - downstream still uses the Phase 0 default
   and ignores the (now non-full-frame) box it's handed.

## Phase 2 — BLUR grows (report-only)

**File:** `engine/src/operations/transform/blur.rs`.

Override `output_bbox()` to grow the `Source` input's box by `BLUR`'s own
kernel radius via `Rect::grow`, clamped to the frame. Still report-only -
`BLUR`'s `execute()` doesn't change in this phase either.

**Acceptance criteria:**
1. Given a `Source` reporting a sub-frame box, `BLUR`'s reported box is
   that box grown by exactly its radius on every side, clamped so it
   never exceeds `Rect::full`.
2. A `Source` already reporting full-frame stays full-frame after growth
   (grow-then-clamp is a no-op at the frame edge).

## Phase 3 — Consume: masked operations get cheaper

This is the phase that delivers the actual compute savings and the
phase where the consume-equivalence invariant (`BBOX_CONVENTIONS.md`)
must be enforced per operation before merging.

**New shared helper**, `graphics/mask.rs`, next to `apply_mask`:

```rust
/// Only invokes `compute` for pixels inside `work_area`; every pixel
/// outside it is copied directly from `original` - the one place
/// "restrict to a bbox, else pass through" is implemented, reused by
/// every masked operation instead of each hand-rolling its own
/// restricted loop.
pub fn compute_within_bbox(
    width: u32, height: u32, work_area: Rect,
    original: &[f32],
    compute: impl Fn(u32, u32) -> [f32; 4],
) -> Vec<f32> { ... }
```

**Operations to migrate, one at a time, each independently mergeable**
(every operation in the tree today with a wired `Input::Mask`):
`blur.rs`, `invert.rs`, `shuffle.rs` (transform); `chromakey.rs`,
`hue_key.rs` (key - note: only their *masking*, i.e. their own wired
`Input::Mask`, is in scope here, not deriving a box from *their own*
keyed-out alpha, which is the excluded Phase 4); `ghost.rs` (generators);
`add.rs`, `screen.rs`, `subtract.rs` (compose).

Per operation: read `find_bbox(&ctx.input_bboxes, Input::Mask)` and the
operation's own natural bbox (its `Source`'s box, or `Rect::full` if
unset), intersect them into `work_area`, and use `compute_within_bbox`
(or the operation's own equivalent restricted loop, if its compute
doesn't fit that helper's per-pixel-closure shape) instead of computing
its full-frame "processed" result unconditionally. The final
`apply_mask` blend step is unchanged - this phase only restricts what
feeds *into* it.

**Acceptance criteria, per operation:**
1. The consume-equivalence test from `BBOX_CONVENTIONS.md`: execute once
   with a real (smaller) `Mask` bbox in `ctx.input_bboxes`, once with an
   empty/full-frame one, assert identical output buffers.
2. A pixel/call-count instrumentation check showing strictly less work
   was done when the mask's box is smaller than full-frame - extend
   `profiling.rs`'s existing `Profile`/`ProfileEntry` (already used by
   `RenderExecutor::execute_profiled`) with an optional pixels-computed
   counter, rather than inventing a separate instrumentation mechanism.
3. A graph-level integration test: a small chain (a geometric mask, e.g.
   `CHECKERBOARD` → `RESIZE` → `MOVE`, wired as `Input::Mask` into the
   operation under test) confirms end-to-end pixel-identical output with
   bbox consumption on vs. off.

Land these one operation at a time - each is its own commit/PR-sized
unit, independently verifiable, matching the incremental rollout
management asked for. Don't batch all eight into one change.

## Out of scope for this round (do not implement)

- Chromakey/hue_key content-derived box tightening (deriving a box from
  an operation's own computed alpha, rather than from parameters/input
  boxes) - see `PARKED_WORK.md`. The `output: &Value` argument on
  `output_bbox()` exists specifically so this can be added later without
  a second trait-signature change; leave it unused this round rather than
  guessing at what that follow-up will need.
- Shrinking/reallocating pixel buffers to bbox size. Every buffer stays
  full-frame-sized always in this round - only *which pixels get
  computed* changes. Buffer-shrinking is a separate, larger change not
  needed for the stated performance goal.
- Building the `CROP` node itself - out of scope per the original
  brainstorm. This work makes `CROP` cheap *once it exists*, using the
  same mechanism as everything else, but doesn't build it.
- Any change to visual output, anywhere, in any phase. Every phase's
  acceptance criteria are pixel-identity checks against today's behavior.
