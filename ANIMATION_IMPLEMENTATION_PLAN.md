# Implementation plan: animation ops, TEXT/GHOST/RING generators, parameter wiring

Working plan for an implementer agent, not a standing convention - delete
or archive this file once the phases below are done (it isn't linked from
`CLAUDE.md`; ask before adding a permanent link if you want it kept as
reference). Assumes `ANIMATION_CONVENTIONS.md` and `PIXEL_CONVENTIONS.md`
have already been read - this plan follows their rules, it doesn't repeat
them.

**Order matters. Do the phases in this order** - each is designed so the
next one's prerequisite work is already done:

1. Phase A - cheap animation-logic operations (Lissajous + 2-3 more), unwired.
2. Phase B - TEXT/GHOST/RING generators, usable standalone.
3. MIX - a new COMPOSE-menu crossfade operation, prerequisite for Phase C
   (see its own section below for why).
4. Phase C - the parameter-wiring mechanism that connects A's outputs to
   MIX's (or any operation's) Number parameters.

Phase A and B have no dependency on each other and can be built in either
order or in parallel; both must exist before Phase C is useful (nothing to
wire yet otherwise), though Phase C's own engine work doesn't technically
require A or B to exist first if you want to build it earlier. MIX has no
dependency on A or B either, but should land before Phase C - see below.

---

## Phase A - cheap animation-logic operations

**Goal:** standalone, pure, unit-tested Number-output operations. They do
not render anything by themselves and are not wired to anything yet - per
CLAUDE.md's "no default anything," a node existing unwired must have zero
effect on the graph's output. Each one becomes a signal source Phase C can
later wire into any Number parameter.

### A1. Add an `Animation` operation category

`engine/src/compositor/metadata.rs` - add a variant to `OperationCategory`:

```rust
pub enum OperationCategory {
    Source,
    Generator,
    Mask,
    Composite,
    Reference,
    Color,
    Animation,   // <-- new
}
```

Add `OperationCategory::Animation => "animation"` to `as_str()`. This is
cheap and honest: these operations produce numbers, not pixels, so lumping
them into `Generator` (which today always means "produces an `Image`")
would be misleading the first time someone reads `category` expecting
that invariant to hold.

### A2. New module

- `engine/src/operations/animate/mod.rs` - `pub mod lissajous;` (and one
  `pub mod` line per op added in A4), plus `pub use lissajous::Lissajous;`
  etc.
- Add `pub mod animate;` to `engine/src/operations/mod.rs` (alongside the
  existing `compose`/`generators`/`key`/`sources`/`transform` lines).

### A3. `Lissajous` - `engine/src/operations/animate/lissajous.rs`

Follow the exact shape of `operations/transform/resize.rs` (struct +
`Operation` impl + `inventory::submit!` + `#[cfg(test)] mod tests` in the
same file).

```rust
pub struct Lissajous {
    pub freq_x: f64,      // default 3.0
    pub freq_y: f64,      // default 2.0
    pub phase_degrees: f64, // default 0.0 - the classic Lissajous phase offset
    pub amplitude: f64,   // default 1.0
}
```

- `descriptor()`: `menu: "ANIMATE"`. This isn't a guess - `ui/scripts/engine/menu.js`'s
  `CATEGORY_ORDER` already has `"ANIMATE"` reserved between `"GENERATE"`
  and `"COMPOSE"`, unused by any operation today. That's pre-existing
  scaffolding for exactly this menu, not something you need to add on the
  JS side.
- `metadata()`: `category: OperationCategory::Animation`, `inputs: vec![]`
  (purely time-driven, no pixel input), `outputs: vec![OutputKind::Number, OutputKind::Number]`
  (X then Y, in that order - Phase C's wiring targets an output by index).
- `parameters()`: `FREQ_X`, `FREQ_Y`, `PHASE` (degrees, step 1, min 0, max
  360), `AMPLITUDE` (step 0.1, min 0) - all `ParameterKind::Number`,
  same pattern as `Resize`'s `SCALE_X`/`SCALE_Y`.
- `execute()`: read `ctx.meta.time` (seconds, already populated - see
  `App::context()` / `start_time_ms` in `app.rs`), compute
  `x = amplitude * (TAU * freq_x * time + phase_radians).sin()`,
  `y = amplitude * (TAU * freq_y * time).sin()`, return
  `Ok(vec![Value::Number(x), Value::Number(y)])`.
- **Must override `is_live() -> bool { true }`.** This is the single
  easiest mistake to make and the one to flag loudest: `RenderExecutor`'s
  cross-tick cache (`executors/render.rs`) keys only on parameter
  fingerprint + resolved inputs, never on `ctx.meta.time`. Lissajous has no
  inputs and (once you stop turning the knobs) unchanging parameters, so
  without `is_live() -> true` its output gets cached after the first tick
  and the animation visibly freezes. `LiveCountingSource` in
  `executors/render.rs`'s own test module, and the real `CameraSource`/
  `VideoSource` operations, are the existing precedent for this - copy the
  pattern, don't reinvent it.
- Tests, mirroring `resize.rs`'s test style:
  - `at_time_zero_with_zero_phase_both_axes_start_at_the_origin`
  - `x_and_y_are_independent_when_frequencies_differ` (assert the two
    outputs actually diverge at some `t`)
  - `amplitude_scales_the_output_range`
  - `is_live_returns_true` (regression guard for the freeze bug above -
    this is the most important test in the file)
  - `set_parameter_rejects_a_negative_amplitude` (mirror `resize.rs`'s
    `set_parameter_rejects_an_out_of_range_scale`)
  - `lissajous_in_graph_is_valid` (mirror `resize.rs`'s own
    `resize_in_graph_is_valid`: `graph.add_node` + `graph.validate()` +
    `RenderExecutor::new().execute(...)`, confirming an unwired Lissajous
    node renders without error)

### A4. Two or three more, same module, same shape, same `is_live()` requirement

Pick based on what's actually wanted visually - don't over-decide this
now. Reasonable starting set, cheapest first:

- **`Sine`** - single output, params `FREQUENCY`/`PHASE`/`AMPLITUDE`/
  `OFFSET`. The simplest possible one; a good first op to write before
  Lissajous if you want an easier warm-up on the pattern.
- **`Square`** (or `Pulse`) - single output, hard on/off wave with a
  `DUTY_CYCLE` param - useful for strobe-like effects a smooth sine can't
  produce.
- **`Noise`** - single output, a deterministic hash-based value noise
  seeded by `ctx.meta.time` (not the `rand` crate - it must stay
  reproducible/testable, same reasoning as everything else in this
  engine being pure). Lowest priority of the three; skip it if "two or
  three" ends up being two.

### A5. Verify

`cargo test` (whole workspace) must pass. Then confirm the WASM build
still compiles (check the repo's existing build script/CI config for the
exact command - don't guess a `wasm-pack` invocation without checking).
Per CLAUDE.md's rule against over-investing in browser test scripts: a
quick manual or Playwright smoke check that the ANIMATE menu now lists
these operations and their Number parameters are editable via the
existing generic parameter-stepper UI is enough - **no JS code should be
needed for this to work**, since `get_operations()` /
`renderOperationList()` are already fully data-driven off each
operation's `descriptor()`/`parameters()`. If the ANIMATE menu does *not*
show them without JS changes, that's a real finding to report, not
something to route around.

---

## Phase B - TEXT / GHOST / RING generators

Each must work standalone, wired to nothing, exactly like `Checkerboard`
(`operations/generators/checkerboard.rs`) - same module
(`operations/generators/`), same `OperationCategory::Generator`, same
`menu: "GENERATE"`.

### RING - real spec (confirmed with the user, supersedes an earlier wrong guess)

An earlier draft of this plan guessed RING's shape from a hypothetical
example in `CLAUDE.md` (per-group colour pickers) and from three old
screenshots in `ui/assets/` (`rings.png`, `key-rings.png`,
`key-rings-2.png`) that turned out to show at least two different,
mutually inconsistent effects (a ripple/warp distortion in one, a plain
generated ring texture in another, an unrelated diagonal-plaid keyed
silhouette in the third) - none of that was a reliable spec, and the
per-group-colour design built on it has been discarded. Use the actual
spec below instead:

- **Static, not animated.** No `is_live()` override, no time dependency
  in `execute()` at all - purely a function of its own parameters, same
  as `Checkerboard`. (Wiring a Phase-A animation op into one of its
  Number parameters later, via Phase C, is a separate, opt-in thing a
  user can choose to do - RING itself has no built-in motion.)
- Exactly four parameters, all `ParameterKind::Number`:
  - `COUNT` - how many concentric rings (step 1, min 1).
  - `RADIUS` - the outer radius of the whole ring set (the "like Saturn's
    rings" size - how far out the pattern extends overall).
  - `SPACING` - the distance between individual rings (only visually
    matters when `COUNT` > 1, but always a real, settable parameter).
  - `THICKNESS` - the stroke width of each ring (uniform across all
    rings - the user's spec doesn't ask for per-ring thickness).
- Centered on the frame (no position parameter unless asked for later -
  don't add one speculatively).
- **Each ring gets its own colour**, confirmed by the user, via a ring
  selector + colour chooser in a deep menu - not a dynamic per-ring
  parameter pool. Two more parameters, both `group: Some("COLOUR")`:
  - `RING_SELECTOR` - `ParameterKind::Number { step: 1.0, min: Some(1.0), max: Some(count) }`.
    The `max` must track the live `COUNT` value (recomputed in
    `parameters()`, not a fixed literal), so the stepper never offers
    more rings than actually exist, per CLAUDE.md rule 2 - exactly the
    same requirement the earlier (discarded) per-group-colour design was
    trying to satisfy, just met a completely different way.
  - `RING_COLOR` - `ParameterKind::Color`. Its `get_parameter`/
    `set_parameter` read/write whichever index `RING_SELECTOR` currently
    points at, out of an internal `Vec<Color>` sized to `COUNT` (grow/
    shrink it when `COUNT` changes via `set_parameter("COUNT", ...)` -
    pick a reasonable fill for newly-added slots, e.g. clone the last
    ring's colour, and say so in a comment rather than leaving it
    unexplained).
  - This is **existing UI infrastructure, not new work**: `menu.js`'s
    `enterParameterGroup()` (the `param_group` `MenuContext`, with UP
    navigating back out) already implements exactly this "deep menu"
    drill-in, and `Checkerboard`'s own `A`/`B` colour parameters already
    use `group: Some("COLOUR")` to render inside one. RING reuses the
    identical mechanism - one stepper (`RING_SELECTOR`) plus one colour
    field (`RING_COLOR`) in that pane, instead of two fixed named colour
    fields. No JS changes needed for the group/drill-in behaviour itself.
  - Always exactly two parameter entries in `parameters()` regardless of
    `COUNT` - this is what makes it simpler than the discarded design,
    which needed a whole fixed-name-pool workaround to stay within
    `ParameterDescriptor.name: &'static str`.
  - Still open: what fills the space *between* rings and outside the
    outermost one - transparent (matching `Resize`'s convention for
    uncovered space) is the reasonable default and doesn't need to be
    asked about separately, but flag it in the PR/commit description
    when implemented so it's a visible decision, not a silent one.
- The old (deleted, pre-node-graph) UI hook was `toggleRingsEnabled` - an
  enable/disable boolean. **Don't reintroduce it.** In this node-graph
  architecture, a node's presence + wiring already is its enable/disable
  (CLAUDE.md rule 3) - an unwired or non-existent RING node already
  produces nothing, the same effect that toggle used to achieve in the
  old non-graph version.
- Pixel generation itself: same shape as `Checkerboard::generate()` - a
  `width`/`height` loop computing each pixel's distance from center,
  testing which ring band (if any) it falls in given `RADIUS`/`COUNT`/
  `SPACING`/`THICKNESS`, `Value::Image` output, no inputs.

### TEXT - one real fork in the road, decide it before writing code

`engine/Cargo.toml`'s `web-sys` features already include
`"CanvasRenderingContext2d"` and `"TextMetrics"` - the latter is enabled
but **used nowhere in the codebase today** (verified: only
`CanvasRenderingContext2d` is actually referenced, in `engine/src/dom.rs`,
for the existing render-boundary canvas write and the video/camera
scratch-canvas pixel read). This is real scaffolding for a
browser-canvas-text approach that was anticipated but never built - not
proof a decision was made, just evidence of which direction was expected.

There are two genuinely different ways to build this, and picking one is
a real design decision, not a detail to smooth over:

1. **Browser Canvas2D text** (what the enabled feature suggests): give
   TEXT a persistent scratch `HtmlCanvasElement` (same injected-resource
   pattern `VideoSource`/`CameraSource` already use via
   `set_pixel_source_on_node` in `app.rs`), call `fillText()`/
   `measureText()` on it with the operation's `TEXT`/`FONT_SIZE`/`COLOR`
   params, read the pixels back. Cheap to build, but it makes TEXT the
   *first* operation whose `execute()` depends on a browser-supplied
   resource rather than being a pure function of `ctx`/`inputs` -
   `cargo test` can't exercise it without a mocked canvas, unlike every
   other operation in this tree (breaks the "prefer Rust unit tests"
   property CLAUDE.md rule 4 relies on).
2. **Pure-Rust rasterization**: embed a small fixed-width bitmap font
   (not a general TTF/outline-font renderer - that's a much bigger,
   not-cheap undertaking on its own) and rasterize glyph coverage
   directly into the pixel buffer, same as `Checkerboard`. Stays 100%
   consistent with every other operation's pure-computation, fully
   `cargo test`-able model, at the cost of a limited character set/no
   real typography.

**Recommendation: option 2, if a small bitmap font for a basic
character set is acceptable** - it's the one that doesn't introduce a
new, untested-by-`cargo-test` category of operation into the tree. But
this is worth confirming with whoever's driving the work before writing
code, since it trades off visual quality against staying inside this
project's existing testing model.

### GHOST - real spec (confirmed with the user, supersedes "don't invent one")

No prior evidence existed anywhere in the repo for this one (unlike
RING, nothing to misread) - the spec below came entirely from asking.
It is a **spatial** repeat/offset effect, not temporal - despite the
word "delay" in how the user first described it, their own worked
example (source at `(0,0)`, one ghost, `spatial = (-1, 0)` -> the ghost
renders left of the source) confirms `DISTANCE` is a spacing amount, not
a time lag. There is no frame-feedback/persistence involved at all.

- **Inputs:** `Input::Source` and `Input::Mask` - both already exist as
  `Input` enum variants (`compositor/input.rs`), no new variant needed.
  The mask picks out "the masked object" that gets repeated; unlike
  every other operation's *optional* `Input::Mask` (used only to blend
  toward an identity via `graphics::mask::apply_mask`), GHOST's `Mask`
  is **required** - there is no sensible "no mask wired" behaviour for
  an operation whose entire job is repeating *the masked region*.
  `metadata().inputs` should reflect that it's not optional the same way
  `Resize` marks `SOURCE` required (an unwired required input already
  shows as `NodeValidation::MissingInput` via existing graph validation -
  no new validation mechanism needed, just don't treat `Mask` as
  optional in this operation's own logic the way `apply_mask`-based ops
  do).
- **Parameters**, all `ParameterKind::Number`:
  - `GHOST_COUNT` - how many ghost copies, in addition to the source's
    own (unmoved) render (step 1, min 0).
  - `DISTANCE` - spacing between consecutive ghosts, and from the source
    to the first ghost - confirmed uniform/linear, not per-ghost.
  - `SPATIAL_X`, `SPATIAL_Y` - direction, same two-`Number` shape as
    `Resize`'s `SCALE_X`/`SCALE_Y` rather than inventing a new vector
    `ParameterKind` for this (no existing `ParameterKind` covers a 2D
    vector - `graphics::geometry::Point2D`/`Center` are unrelated, unused
    MOVE scaffolding, not a fit here).
  - `OPACITY_MULTIPLIER` - **one shared value, applied identically to
    every ghost - not indexed by `n`, no per-ghost formula.** Corrected
    from an earlier draft of this plan, which wrongly turned "opacity
    changes by a step property" into a progressive `1 + n * step`
    formula that varied per ghost - the user explicitly did not say
    that; every ghost `n` in `1..=GHOST_COUNT` uses this exact same
    multiplier, clamped `0.0..1.0`. "Step" here just means the ordinary
    UI increment on a `Number` parameter (the `step` field every other
    `ParameterKind::Number` already has, e.g. `Resize`'s `SCALE_X`), not
    a per-ghost math term. The source itself (`n = 0`) always renders at
    its own native opacity, unaffected by this parameter - it's the
    reference point, same as it is for `DISTANCE`.
- **Position formula** (fully specified by the user's own example):
  `ghost_n_offset = n * DISTANCE * (SPATIAL_X, SPATIAL_Y)` for
  `n = 1..=GHOST_COUNT`.
- **Execute algorithm:**
  1. Resolve `Source` and `Mask` via `FloatImage::from_value`/
     `graphics::mask::resolve_pixels`, same as any masking-capable
     operation.
  2. Isolate the masked object as its own standalone RGBA buffer (source
     colour, alpha = source alpha × mask alpha, transparent outside the
     mask). **This is a new pixel helper, not something `apply_mask`
     already does** - `apply_mask` blends two already-computed *results*
     toward each other by mask weight; GHOST needs a cutout/stencil
     extraction, a different operation. Small and self-contained -
     doesn't belong in `graphics::mask` as-is, but could be added there
     as a sibling function if it turns out generally useful.
  3. For each `n` in `1..=GHOST_COUNT`: translate the cutout buffer by
     `ghost_n_offset` (nearest-neighbor shift with transparent padding at
     the vacated edge - same inverse-mapping shape `Resize::resize_pixels`
     already uses, just a translation instead of a scale), then multiply
     its alpha channel by `OPACITY_MULTIPLIER` (the same value for every
     ghost).
  4. Composite the source's own cutout (`n = 0`, full opacity) and all
     `n = 1..=GHOST_COUNT` ghost layers into one output image. **Stacking
     order for overlapping ghosts is a real open question the user
     hasn't specified** - nearest-to-source-on-top (paint far-to-near,
     i.e. highest `n` first) is the reasonable default and matches how a
     real motion trail reads, but say so explicitly wherever this is
     implemented rather than leaving the choice silent.
  5. Per `PIXEL_CONVENTIONS.md`: there is **no real front-to-back "over"
     operator in this engine yet** - only uniform-channel `Add`/
     `Multiply`/`Screen` and the narrow `apply_mask` blend-toward-original
     helper, neither of which is what stacking N alpha-varying, spatially
     offset layers needs. GHOST's own compositing step is a legitimate
     reason to write a real alpha-over helper (standard
     `result = fg * fg_a + bg * (1 - fg_a)`, applied N times in stacking
     order) - but per `PIXEL_CONVENTIONS.md`'s own rule, that must be
     written as its own explicit thing (e.g. a small internal helper
     function GHOST's `execute()` calls), **not** a retrofit of
     `Add`/`Multiply`/`Screen`'s existing uniform-channel semantics, which
     stay exactly as they are.
  6. No extra gamut-safety logic needed in that helper: alpha-over is a
     convex combination (`fg * a + bg * (1-a)`) weighted by an opacity
     already clamped to `0.0..1.0` (`OPACITY_MULTIPLIER`), so
     it can't *introduce* an out-of-gamut result the way `Add`/`Multiply`/
     `Screen` can. If `Source` itself is already out-of-gamut
     (FloatImage from an upstream `ADD` chain, say), that can still carry
     through the blend - but that's the same "blending toward an
     out-of-gamut value can still be out of gamut" behaviour
     `mask.rs`'s own tests already establish as correct, not a new case
     to special-case here.

---

## MIX - a crossfade operation, prerequisite for Phase C

**Why this exists:** surveying every real operation in the tree (see the
conversation this plan was drafted from) found that roughly half of
them - `MULTIPLY`, `ADD`, `SCREEN`, `SUBTRACT`, `INVERT`, `RGB_TO_HSV`,
`SHUFFLE`, `IMAGE`/`VIDEO`/`CAMERA` sources - have **zero** eligible
`Number`-kind parameters, i.e. nothing Phase C's wiring mechanism could
ever attach to. MIX sidesteps that entirely: it doesn't matter whether
the two things it's blending have any parameters of their own, because
MIX supplies exactly one, purpose-built for animation to drive.

**Not a revival of the old "no MIX node" decision.** Commit `d472e0e`
("Add a generic MASK input, no MIX node") solved a different problem -
modulating *one operation's own effect strength* using another node's
alpha as a per-pixel weight (already covered by the generic `MASK`
input). MIX is a different capability: crossfading between **two
independent pixel sources** by a single uniform amount. Nothing in the
tree does that today.

**Spec:**
- `descriptor()`: `menu: "COMPOSE"`. `metadata()`: `category:
  OperationCategory::Composite`, `inputs: vec![Input::Foreground,
  Input::Background]` (both required - error like `Screen` does if
  either is missing), `outputs: vec![OutputKind::FloatImage]`.
- Exactly one parameter: `AMOUNT` - `ParameterKind::Number { step: 0.01,
  min: Some(0.0), max: Some(1.0) }`. Clamped in `set_parameter`, same
  reasoning as `GHOST`'s `OPACITY_MULTIPLIER` and `apply_mask`'s mask
  weight: this is a blend weight, not a light value, so an out-of-range
  value has no sensible meaning to preserve the way an out-of-gamut
  colour does (see the alpha-clamping discussion earlier in this
  session, and `PIXEL_CONVENTIONS.md`'s alpha section).
- `execute()`: resolve `Foreground`/`Background` via
  `FloatImage::from_value`, check matching dimensions (mirror `Screen`'s
  check, same error), then per pixel, **all four channels uniformly**
  (matching `Add`/`Multiply`/`Screen`'s existing convention - this is a
  plain crossfade, not `GHOST`'s alpha-aware Porter-Duff "over", so it
  does not need a dedicated alpha formula the way `GHOST`'s compositing
  step did):
  `output[c] = foreground[c] * (1.0 - amount) + background[c] * amount`
  for `c` in `0..4`.
- No extra gamut-safety logic needed, same reasoning as `GHOST`'s
  alpha-over helper: this is a convex combination weighted by an
  already-clamped `0.0..1.0` amount, so it cannot introduce an
  out-of-gamut result on its own - inputs that are already out-of-gamut
  can still carry through, which is correct, established behaviour, not
  a new case.
- Tests: `amount_zero_is_pure_foreground`, `amount_one_is_pure_background`,
  `amount_half_averages_both_inputs`, `set_parameter_rejects_an_amount_above_one`,
  `execute_errors_on_mismatched_dimensions`, `mix_in_graph_requires_both_inputs`
  (mirror `Screen`'s and `Multiply`'s existing test shapes).

**Worked example** (also see the node-flow diagram and pixel-level walk-
through in the conversation this plan was drafted from): two `RING`
nodes at different static `RADIUS` values wired into `MIX`'s
`Foreground`/`Background`, with `MIX.AMOUNT` driven by `SINE` (once
Phase C exists) - the render smoothly crossfades between "small ring
visible" and "large ring visible" without either `RING` node's own
`RADIUS` ever being touched.

---

## Phase C - parameter wiring (IMPLEMENTED)

Built, tested (179 Rust tests passing, native + `wasm32-unknown-unknown`
both build clean), and verified live in a browser: wired `SINE` into
`MIX.AMOUNT` and confirmed the render actually animates over real time
(three screenshots at different moments show the crossfade sweeping).
The design below **supersedes an earlier draft of this section**, which
had the consumer (the target) own the wire and required widening
`Execute::execute` from `&Graph` to `&mut Graph`. Mid-design, direction
flipped (see the conversation this plan was drafted from: "in the ui i
want to keep that nodes manipulate the inputs, no node should select
its output as property") - the **driver** now owns the connection and
authors it from its own edit screen, which turned out to avoid the
`&mut Graph` widening entirely. What actually shipped:

### Storage - on the driver, not the target

`compositor/graph/node.rs`'s `Node` gained two fields:

```rust
pub animation_target: Option<NodeId>,
pub animation_mappings: Vec<(usize, &'static str)>, // (output_index, target_parameter_name)
```

One target node per driver (not per output - "select source" is a single
pick), and a sparse list mapping each of the driver's own output indices
to a parameter name on that one target. `&'static str`, matching
`ParameterDescriptor.name` exactly like the discarded design intended -
resolved and validated once at connect-time, never a caller-supplied
owned string stored long-term.

### Graph API - `compositor/graph/edit.rs`

- `connect_animation_target(driver, target)` - rejects `driver == target`,
  rejects a driver whose own `metadata().category != OperationCategory::Animation`,
  and rejects a `target` that's *itself* Animation-category. That last
  rule is what makes chained/cyclic driving structurally impossible
  rather than something to detect: `validate.rs`'s cycle DFS did not
  need touching at all, because a driver can never point at another
  driver in the first place. Clears `animation_mappings` on every
  target change (they named parameters on the *old* target).
- `disconnect_animation_target(driver)` - clears target and all mappings.
- `set_animation_mapping(driver, output_index, target_parameter: &str)` -
  validates `output_index` against the driver's own declared output
  count, and `target_parameter` against the target's *current, real*
  `ParameterKind::Number` parameters, storing the matched `&'static str`
  from the target's own `ParameterDescriptor` (never the caller's `&str`).
- `clear_animation_mapping(driver, output_index)`.
- `remove_node` now also clears `animation_target`/`animation_mappings`
  on any node whose target was just removed, same spirit as its existing
  `inputs.retain(...)` cleanup for pixel wires.

### Execution - a flat pre-pass, not a widened `Execute` trait

`compositor/graph/drive.rs` (new file, mirrors the `describe`/`edit`/
`resolve`/`validate` split) - `Graph::apply_animation_drivers(&mut self,
ctx: &Context)`:

- Scans every node once for `animation_target.is_some() &&
  !animation_mappings.is_empty()`, calls that driver's own `execute(ctx, &[])`
  (drivers declare zero pixel inputs - see Phase A - so no input
  resolution is needed), and for each `(output_index, target_parameter)`
  pushes the corresponding output value into the target via its
  **already-existing** `set_parameter()` - exactly the injection point
  `ANIMATION_CONVENTIONS.md` decided on, just fed by a wired driver
  instead of an (unbuilt) authored curve.
- A **flat** pass, not a recursive DAG walk, on purpose: since a driver
  can never target another driver (enforced above), there is no
  ordering dependency between drivers to resolve - every driver can be
  evaluated in any order.
- Called once per tick from `App::render_tick` *and* `App::preview_tick`
  (`app.rs`), *before* the normal executor call - so `RenderExecutor`'s
  existing cache (which already fingerprints a node via `get_parameter()`
  on every declared parameter) picks up the animated change for free.
  **This is why `Execute::execute`'s `&Graph` signature never needed
  widening**: the mutation happens in a separate step before the normal
  immutable DAG walk, not inside it.
- Infallible by design: a `set_parameter()` rejection (an animated
  value momentarily outside the target's valid range) is silently
  dropped for that tick rather than propagated as an error - the target
  keeps its last value, the whole render never blanks over one
  out-of-range tick. Same reasoning as a human typing a bad value by
  hand being rejected, not crashing anything.
- What actually did need adding, orthogonal to the above: `metadata().outputs`
  already being `Vec<OutputKind>` was never the blocker C0's original
  draft worried about - `execute()` already returns the full `Vec<Value>`
  and this pre-pass reads it directly by index, no executor-level
  multi-output plumbing required. What *was* missing: a way to *label*
  each output ("X"/"Y", not just positionally distinguishable) for the
  UI. Added `Operation::output_names() -> Vec<&'static str>` as a new
  **default-provided** trait method (`Vec::new()` by default - purely
  cosmetic, nothing in the engine depends on it) - overridden by
  Lissajous (`["X", "Y"]`), Sine and Square (`["OUTPUT"]`). This is a
  small, additive trait extension, not the kind of change
  `ANIMATION_CONVENTIONS.md`'s "never touch `Operation`/`ParameterKind`"
  rule was guarding against (that rule is about the parameter
  get/set/describe path specifically staying the one hook for state).

### WASM bindings - `app.rs`

Mirrors `connect_node_input`/`disconnect_node_input`/`node_inputs` in
shape: `node_outputs(node_id)` (index + label per declared output),
`animation_target(node_id)` (current target or `None`),
`animation_mapping(node_id, output_index)` (current mapping or `None`),
`connect_animation_target`, `disconnect_animation_target`,
`set_animation_mapping`, `clear_animation_mapping`. No binding needed
for "list eligible targets" or "list eligible target parameters" -
both are derived in JS from the already-existing `node_parameters()`
(filtered to `kind === "NUMBER"`) and the already-cached
`menuManager.operations` (filtered to exclude `category === "animation"`).

### JS UI - a new deep-menu pane, authored from the driver

`ui/scripts/engine/nodeEditContexts.js`:

- `renderGenericEditContext` (the default edit-screen renderer every
  node without a bespoke context gets) now adds an `INPUT >` button -
  but only when the node's own operation is Animation-category (checked
  via `menuManager.operations`, never a hardcoded id list).
- New `renderAnimationTargetContext(menuManager, nodeEntry)`: a `TARGET`
  stepper (`NONE` + every eligible node, same shape as
  `renderInputSteppers`'s candidate list), and - only once a target is
  picked, so there's never a dead "CONTROLS" row with nothing real to
  offer - one `"<OUTPUT NAME> CONTROLS ___"` stepper per output the
  driver declares, each bounded by the *current* target's real Number
  parameters.

`ui/scripts/engine/menu.js`:

- `CONTEXT_HANDLERS` gained `animation_target: "renderAnimationTargetContext"`.
- New `MenuManager.renderAnimationTargetContext()` method (mirrors
  `renderParamGroupContext`) and `enterAnimationTarget()` navigation
  method (mirrors `enterParameterGroup`, minus the group-name argument -
  there's only ever one "INPUT" pane per driver).

### Verified

`cargo test` (4 new tests in `drive.rs`, plus the full suite - 179
passing), `cargo build --target wasm32-unknown-unknown` clean, and a
real browser session: created two `RING`s, a `MIX` wired to both, a
`SINE` tuned to sweep `0..1` (`OFFSET=0.5, AMPLITUDE=0.5`), drilled into
`SINE`'s `INPUT` pane, picked `MIX 1` as `TARGET`, mapped `OUTPUT
CONTROLS -> AMOUNT`, set `MIX` as `LIVE`, and screenshotted the render
at three points in time - the crossfade visibly swings between "mostly
Ring 1" and "both rings blended" as `SINE` oscillates, live.

---

## Explicitly out of scope for all three phases

Authored keyframing (a human placing keyframes on a timeline, curve
interpolation/extrapolation UI) is a separate, larger effort - see
`ANIMATION_CONVENTIONS.md`'s "explicitly not decided yet" list. Nothing
in this plan blocks it: C4's injection point (resolve-a-value-then-
`set_parameter()`) is the same hook a future curve evaluator would use,
just with a curve sample instead of a wired node's output as the value
source.
