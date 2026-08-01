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
3. Phase C - the parameter-wiring mechanism that connects A's outputs to B's (or any operation's) parameters.

Phase A and B have no dependency on each other and can be built in either
order or in parallel; both must exist before Phase C is useful (nothing to
wire yet otherwise), though Phase C's own engine work doesn't technically
require A or B to exist first if you want to build it earlier.

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
  - `OPACITY_STEP` - confirmed: signed (can be negative *or* positive),
    added once per ghost step. Ghost `n`'s opacity multiplier is
    `(1.0 + n * OPACITY_STEP).clamp(0.0, 1.0)` for `n` in `1..=GHOST_COUNT`
    (the source itself, `n = 0`, always renders at its own native
    opacity - it's the reference point, same as it is for `DISTANCE`).
    Positive fades ghosts *in* with distance, negative fades them *out* -
    both are real, user-facing choices, not a hardcoded direction.
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
     its alpha channel by that ghost's opacity multiplier.
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

---

## Phase C - parameter wiring (connect Phase A's outputs to any Number parameter)

This is the piece that turns "Lissajous exists as a node" into "Lissajous
can drive RING's radius." It's real engine work, not just UI - four
sub-parts, in dependency order.

### C0. Prerequisite you will hit immediately: multi-output values don't actually work today

`metadata().outputs` is already `Vec<OutputKind>` "so a future
multi-output operation doesn't need the shape to change" (see the comment
on `OutputKind` in `compositor/metadata.rs`) - but that promise isn't
kept yet. `RenderExecutor::evaluate()` (`executors/render.rs`) always
does `outputs.into_iter().next()`, throwing away everything after index
0, and both its per-tick cache (`CachedNode.value: Value`) and its
recursion memo (`HashMap<NodeId, Value>`) only ever store one `Value`
per node. Lissajous's Y output (index 1) is unreachable through the
executor as it stands today, even though `execute()` itself already
returns both. Fix this first:

- `CachedNode.value: Value` -> `CachedNode.values: Vec<Value>`
- `memo: HashMap<NodeId, Value>` -> `HashMap<NodeId, Vec<Value>>`
- `evaluate()`/`evaluate_profiled()` return `Vec<Value>` instead of a
  single `Value`; callers that only ever wanted the first value (the
  existing pixel-consuming call sites) take `.into_iter().next()` /
  `[0]` at their own call site instead of inside `evaluate` - this keeps
  every *existing* single-output operation working unchanged.
- No `Operation` impl changes - `execute()` already returns `Vec<Value>`,
  this only touches how the executor stores/threads that vec through.
- Existing tests in `executors/render.rs`'s `#[cfg(test)] mod tests`
  should keep passing with mechanical updates only (they already build
  `graph` as `mut`, so `.execute(&graph, ...)` callers don't need new
  bindings, just signature updates alongside C3 below).

### C1. Graph-level storage for parameter wires

`compositor/graph/node.rs` - add to `Node`:

```rust
pub struct Node {
    pub operation: Box<dyn Operation>,
    pub inputs: Vec<(Input, NodeId)>,
    pub parameter_wires: Vec<(&'static str, NodeId, usize)>, // (param name, source node, output index)
}
```

Deliberately `&'static str`, matching `ParameterDescriptor.name`, not an
owned `String` - resolved and validated once at connect-time (C2) against
the target operation's own `parameters()` list, the same way `Input` (a
fixed `Copy` enum) is validated today. This avoids threading owned
strings through the render hot path for something that's always actually
one of a small fixed set of `&'static str`s per operation.

### C2. Graph API - `connect_parameter` / `disconnect_parameter`

`compositor/graph/edit.rs` - mirror `connect`/`disconnect` exactly:

```rust
pub fn connect_parameter(
    &mut self,
    node: NodeId,
    parameter: &str,
    source: NodeId,
    output_index: usize,
) -> Result<(), OperationError> {
    // 1. resolve(node) and resolve(source) must both exist (OperationError::UnknownNode)
    // 2. look up `parameter` in node's operation.parameters(); reject unknown
    //    names (new OperationError variant, or reuse UnknownParameter)
    // 3. reject if that parameter's ParameterKind is not Number - start
    //    restrictive (see ANIMATION_CONVENTIONS.md's "explicitly not
    //    decided yet" list; Boolean/Enum/Color wiring is future scope,
    //    not this pass)
    // 4. store the *matched* &'static str from the descriptor (not the
    //    caller's &str) in parameter_wires, replacing any existing wire
    //    for that parameter name (same retain-then-push pattern `connect` uses)
}

pub fn disconnect_parameter(&mut self, node: NodeId, parameter: &str) -> Result<(), OperationError>
```

Both must set `self.validation = ValidationState::Dirty;`, same as
`connect`/`disconnect` - a parameter wire is a graph-structural edit and
has to trigger revalidation for C3 to be safe.

### C3. Extend cycle detection to parameter wires

`compositor/graph/validate.rs`'s `visit_cycle_detection` currently only
walks `node.inputs`. A parameter wire is a real dependency edge too - a
Lissajous (hypothetically) wired into its own parameter, or a longer cycle
formed purely through parameter wires, must be caught here, or
`RenderExecutor`'s parameter-resolution step (C4) will recurse forever
the first time someone builds one. Add a second loop over
`node.parameter_wires` right after the existing `for (_, input) in
&node.inputs` loop, feeding the same `state`/`path`/`cycle_nodes`
machinery. Also extend the `unknown_input_nodes` detection loop earlier
in `run_validation` and the `dependents` construction in
`propagate_invalidity` the same way - a parameter wire pointing at a
removed node needs `UnknownInput`-equivalent handling, not a silent
no-op. (Note: this file's own comment already documents a pre-existing
stale-DFS-state bug affecting `state`/`path` reuse across top-level
roots, tracked in `PARKED_WORK.md` - don't fix that as a side effect
here, it needs its own regression test per that entry's own notes.)

### C4. Resolve wires before `execute()` - reuses the exact hook `ANIMATION_CONVENTIONS.md` already decided

This is the one signature change with real blast radius, so do it
deliberately: `Execute::execute` (`executors/mod.rs`) and both impls
(`RenderExecutor`, `PreviewExecutor`) currently take `graph: &Graph`.
Injecting a wire-resolved value via the operation's existing
`set_parameter()` needs mutable access to the node, so this must become
`graph: &mut Graph`. Blast radius, fully enumerated (checked against the
current tree, not guessed):

- `Execute` trait definition - one line.
- `RenderExecutor::evaluate`/`evaluate_profiled` and
  `PreviewExecutor::execute` - signatures only; `PreviewExecutor` doesn't
  need to *do* anything with parameter wires in this pass if you want to
  scope C4 to the render path first, but it must still compile against
  the trait.
- Two call sites in `app.rs`: `render_tick` (~line 533) and
  `preview_tick` (~line 581), both already `&mut self` methods on `App` -
  changing `&self.graph` to `&mut self.graph` there is a non-breaking,
  mechanical change, not a structural one.
- Existing tests in `executors/render.rs` / `executors/preview.rs` that
  call `.execute(&graph, ...)` directly - mechanical `&graph` ->
  `&mut graph` at each call; every one of those tests already declares
  `let mut graph = ...`, so no new `mut` bindings are needed, just the
  call-site edit.

With that done, in `RenderExecutor::evaluate` (after resolving
`input_values`, before computing `param_fingerprint`):

```rust
for &(param_name, source_id, output_index) in &node_data.operation_parameter_wires() {
    // evaluate() the source node recursively (same memo/cache path
    // pixel inputs already use), pull `output_index` out of its Vec<Value>
    // (this is exactly what C0 made possible), and:
    graph.resolve_mut(node).unwrap().operation.set_parameter(param_name, value)?;
}
```

Because this genuinely mutates the operation's stored parameter value
(not a side-channel override), the *existing* cache-invalidation logic in
`RenderExecutor` - which already fingerprints parameters via
`get_parameter()` after every `execute()` - picks up wire-driven changes
for free. No special-casing needed there; this is the payoff of storing
the resolved value through the real `set_parameter()` call instead of
inventing a parallel "override" path.

Order of operations within `evaluate()` matters: resolve parameter wires
*before* computing `param_fingerprint`, so the fingerprint reflects the
value the wire just set, not last tick's stored value.

### C5. WASM bindings

`app.rs` - mirror `connect_node_input`/`disconnect_node_input` (~line
347-390) exactly:

```rust
pub fn connect_node_parameter(&mut self, node_id: u32, parameter: String, source_id: u32, output_index: u32) -> Result<(), JsValue>
pub fn disconnect_node_parameter(&mut self, node_id: u32, parameter: String) -> Result<(), JsValue>
```

Also extend `ParameterView` (or add a sibling field) so
`node_parameters()` reports whether a given parameter currently has a
wire attached and to which node/output - `renderInputSteppers` in
`menu.js` needs this to show current state, same as `InputView.source`
does for pixel wires today.

### C6. JS UI - reuse `renderInputSteppers`'s exact pattern, don't invent a new one

`ui/scripts/engine/menu.js`'s `renderInputSteppers` (~line 372) is
already fully generic: query the node's real inputs from the graph
(never hardcoded), offer NONE + every other real node as stepper options,
call `connect_node_input`/`disconnect_node_input`. Write
`renderParameterWireSteppers` as a straight copy of that shape, querying
`wasmApp.node_parameters(nodeId)` filtered to Number-kind parameters
(from C5's extended `ParameterView`) instead of `wasmApp.node_inputs`,
and calling `connect_node_parameter`/`disconnect_node_parameter`. Hook it
into whatever renders a node's parameter list today (near
`renderInputSteppers`'s own call site) so a Number parameter shows both
its normal stepper *and* a "wire source" option - exact UI arrangement
(combined row vs. separate row) is a small enough call to make while
implementing, not worth deciding here.

### C7. Verify

- `cargo test` - all of C0-C4's changes should be provable at the Rust
  level per CLAUDE.md rule 4: a graph-level test wiring a
  `ConfigurableSource`-style stub's Number parameter to a stub signal
  source, ticking twice, and asserting the parameter's resolved value
  changed between ticks (the animate-ops equivalent of
  `changing_a_parameter_forces_re_execution_next_tick`, already in
  `executors/render.rs`) is the key regression test to add.
- A cycle-through-parameter-wire test mirroring
  `a_genuine_cycle_is_still_flagged_and_still_fails_validation` in
  `validate.rs`, but wiring node A's parameter to node B and B's
  parameter (or input) back to A.
- Only after that: a manual/Playwright smoke check that wiring Lissajous
  into a RING group's radius (or similar) actually animates in the
  browser - diagnostic only, per CLAUDE.md rule 4, not a committed test
  suite.

---

## Explicitly out of scope for all three phases

Authored keyframing (a human placing keyframes on a timeline, curve
interpolation/extrapolation UI) is a separate, larger effort - see
`ANIMATION_CONVENTIONS.md`'s "explicitly not decided yet" list. Nothing
in this plan blocks it: C4's injection point (resolve-a-value-then-
`set_parameter()`) is the same hook a future curve evaluator would use,
just with a curve sample instead of a wired node's output as the value
source.
