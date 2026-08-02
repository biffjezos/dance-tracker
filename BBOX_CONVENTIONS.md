# Bounding-box (bbox) awareness convention

This file records the architecture decisions for making the graph's render
cost scale with "how much of the frame is actually in play" instead of
purely with node count and frame resolution - a scaling concern raised by
management, not a bug fix. Nothing in this file is implemented yet - this is
the shape decision, made once, so implementing it (and every operation that
follows it afterward) doesn't mean re-deriving or retrofitting the same
questions per operation. See `CLAUDE.md` for how this file relates to the
rest of the project's standing rules, and `PIXEL_CONVENTIONS.md`/
`ANIMATION_CONVENTIONS.md` for the sibling decisions made the same way.

## Background

Today, every node in the graph always computes over the full frame buffer,
regardless of how much of that buffer is actually visible/relevant by the
time it reaches the output - a MASK blends its result back against the
original only *after* fully computing that result over the whole frame;
nothing is skipped. As graphs grow, this means render cost grows with node
count alone, with no way for a user's own choices (cropping to a region,
masking most of the frame away) to make things cheaper. The fix: give every
node's output an associated "region that actually matters" - a bounding
box - that downstream nodes can use to do less work, the same pattern
established compositors (Nuke, Fusion) already use.

## Decision: a bounding box is a single axis-aligned rectangle, never richer

**Decision:** `Rect { x0, y0, x1, y1 }` (half-open pixel-index bounds:
`[x0,x1) x [y0,y1)`), nothing more expressive - no polygon, no per-pixel
coverage mask, no rotation.

**Why:** an exact fit isn't the goal - a *safe, conservative* fit is (see
the correctness invariant below). A rectangle is what every relevant
combine operation (shrink, grow, union) needs to stay O(1) and trivially
correct; anything richer (e.g. tracking a diagonal or feathered mask's true
shape) would cost real complexity for a case this round explicitly doesn't
require. A box larger than the true extent of an operation's content is
always an acceptable outcome; a box smaller than it is not.

## Decision: reporting a box and consuming a box are two independently-adoptable steps

**Decision:** every operation, via a defaulted trait method, *reports* its
own output's bounding box. Separately, and optionally, an operation may
*consume* the boxes its own inputs reported to actually skip real
per-pixel work. These are deliberately two different levels of commitment:

- **Reporting** is always safe. The default (full-frame) is exactly
  today's behavior; overriding it to something tighter (a `RESIZE` at 50%,
  a `BLUR` grown by its kernel radius) changes only *metadata* passed to
  downstream nodes, never a single output pixel. An operation can adopt
  this with zero risk to correctness.
- **Consuming** is where an actual bug could clip real content a user
  would otherwise see. This should only be done using the invariant below,
  verified by the paired equivalence test it describes, one operation at a
  time.

This split is what makes an incremental, per-node-category rollout
possible without a single big-bang change: every operation is safe from
the moment the mechanism lands (all defaults), and each operation opts
into *reporting* before it opts into *consuming*, never the reverse.

## Decision: the one correctness invariant everything else depends on

> **For any pixel outside the `Rect` an operation reports as its
> `output_bbox()`, that operation's actual output at that pixel must be
> `[0,0,0,0]`** (fully transparent black - every channel's zero/default
> value). A reported box may be larger than strictly necessary (safe, just
> less optimal); it must never be smaller than the true extent of
> non-default content (unsafe - silently clips real output).

A second invariant covers the consuming half specifically:

> **For any operation that restricts its own compute to a bbox, its
> output must be pixel-identical to running the same operation
> unrestricted (full-frame) on the same inputs.**

Both are meant to be checked by shared, reusable test helpers (not
bespoke-per-operation assertions) - the first named something like
`assert_bbox_contains_all_nonzero_pixels(value, bbox)`, the second a
paired "run with a real bbox vs. run with full-frame, assert equal buffers"
test shape, both reused by every operation that opts in.

## Decision: where this lives in the type system

- **`Rect`** (`compositor/bbox.rs`, new file): `full`/`empty`/`is_empty`/
  `intersect`/`union`/`grow`. `union` treats an empty operand as "ignore
  it," not "extend to include the origin."
- **`find_bbox(bboxes: &[(Input, Rect)], key: Input) -> Option<Rect>`**
  (`compositor/input.rs`, next to the existing `find_input`), same shape.
- **`Operation::output_bbox`** (`compositor/operations.rs`), a new
  defaulted trait method - same pattern as `is_live()`:

  ```rust
  fn output_bbox(&self, ctx: &Context, input_bboxes: &[(Input, Rect)], output: &Value) -> Rect {
      Rect::full(ctx.meta.width, ctx.meta.height)
  }
  ```

  `output` (the operation's own just-computed value) is part of the
  signature from the start even though this round's operations don't need
  it (their boxes are derivable from parameters/input boxes alone, no
  pixel inspection required) - a future content-derived box (an operation
  whose relevant region can only be known from its own computed pixels,
  e.g. a keying operation's actual alpha footprint) is explicitly deferred
  to a separate follow-up round, tracked in `PARKED_WORK.md`, and this
  keeps the trait signature from having to change a second time when that
  round happens.

- **`Context.input_bboxes: Vec<(Input, Rect)>`** (`compositor/context.rs`),
  a new field, not a new `execute()` parameter. `Context` already
  `#[derive(Default)]`s and is constructed via `..Default::default()`
  throughout the existing test suite - adding a field there is invisible
  to every test that doesn't care about it. A new `execute()` parameter
  would instead break every direct `operation.execute(&ctx, &inputs)` call
  site across every operation's tests for zero benefit to operations that
  never read it. The tradeoff taken knowingly: `Context` stops being
  fully tick-uniform - this one field varies per node-call within a tick,
  set by the executor immediately before each `execute()` call, unlike
  `meta`/`resources` which stay identical for every node in a tick. Worth
  remembering during review; still far cheaper to roll out than the
  alternative.
- **Both executors** (`RenderExecutor` and `PreviewExecutor` -
  `compositor/executors/{render,preview}.rs`) thread `(Value, Rect)`
  pairs through their existing recursive evaluation instead of bare
  `Value`, and construct the per-node `Context` (base clone +
  `input_bboxes` override) immediately before calling `execute()`. The
  **public** `Execute::execute()` signature is unchanged - this threading
  is a private implementation detail of each executor's own recursion,
  invisible to `app.rs` and the WASM boundary.

## What this means for new operations

- **Nothing, by default, forever.** An operation that never overrides
  `output_bbox()` and never reads `ctx.input_bboxes` is exactly as correct
  as it is today - the default is always safe. Most operations should stay
  exactly like this until there's a real reason not to.
- An operation whose spatial extent is fully determined by its own
  parameters and its inputs' own boxes (a scale/offset transform, a
  kernel-radius-driven blur, a geometric crop) should override
  `output_bbox()` to report a tighter box - always safe to do on its own.
- An operation that wants the actual compute savings should read
  `ctx.input_bboxes` (via `find_bbox`) and restrict its own per-pixel work
  to the relevant intersected region, verified by the consume-equivalence
  invariant above before it ships. For any operation with a wired `MASK`
  input, use the shared `compute_within_bbox` helper
  (`graphics/mask.rs`, next to `apply_mask`) rather than hand-rolling a
  restricted loop - the single reusable mechanism for "only compute inside
  this region, copy the original everywhere else."
- **Content-derived boxes are out of scope for this round.** An operation
  whose relevant region can only be known by inspecting its own computed
  pixels (a keying operation's actual keyed-out silhouette, which has no
  fixed shape and can change every frame on video) is a separate, harder
  problem - deliberately deferred, tracked in `PARKED_WORK.md`, not
  something to improvise per-operation in the meantime.

## Current state

Nothing in this file is implemented yet - this is the decision, not the
patch. This round's scope is `RESIZE`, `MOVE` (once built), `BLUR`, and
geometric (non-content-derived) masks - see the accompanying
implementation spec for the phased rollout. Content-derived tightening for
`chromakey`/`hue_key` is intentionally excluded from this round; see
`PARKED_WORK.md`.
