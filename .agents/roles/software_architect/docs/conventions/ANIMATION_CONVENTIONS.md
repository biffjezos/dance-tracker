# Animation conventions (keyframing)

This file records one core decision: how time-based parameter animation
(keyframing) will plug into the engine when it's eventually built, so
building it later doesn't mean retrofitting every operation that exists
by then. Nothing in this file is implemented yet - this is the shape
decision, made once while the operation count is still small, not a
description of working code. See `CLAUDE.md` for how this file relates
to the rest of the project's standing rules, and `PIXEL_CONVENTIONS.md`
for the sibling colorspace/alpha decisions made the same way.

## How real compositors do this (Nuke, Fusion, After Effects)

A parameter is never "just a value" - it's something you ask for a value
*at a given time*. Nuke's knobs expose `getValueAt(time)`: depending on
the knob's own state, that returns a static default, samples an
animation curve (keyframes + interpolation), or evaluates an expression
that can reference other knobs/time. The node's own compute logic never
knows or cares which - it asks "what's my value right now" and gets a
plain answer, exactly as if someone had typed it in for that frame only.

Critically, the curve is **not** part of the node's compute code - it's a
separate object the parameter delegates to (Fusion calls these
"Modifiers": literally detachable from the parameter). This is what makes
it retrofittable in those tools without rewriting every node's math.

## Decision: keyframe state lives at the graph layer, never inside an Operation

When implemented, a curve will be graph-owned state keyed by
`(node_id, parameter_name)` - not a field on any `Operation` impl, and
not a new `Operation` trait method. Each tick, before `execute()` runs,
the graph resolves each animated parameter by evaluating its curve at
`ctx.meta.time` (or `ctx.meta.frame`) and calling that operation's
**already-existing** `set_parameter()` - the exact same call the UI makes
today when a person types a value in by hand. `execute()` itself never
learns animation exists; a keyframed `SCALE_X` on `Resize` looks
identical, from inside `Resize`, to someone hand-editing the stepper
every frame.

**Why this is the only rule that needs deciding now:** it means the
entire feature can land later by touching the graph/node layer and the
UI (a curve editor) alone - zero changes to any operation that follows
the rule below. The cost of *not* deciding this now isn't a missing
struct field (like colorspace) - it's an architectural assumption that,
if violated by operations added between now and then, forces revisiting
every violator individually instead of writing the feature once.

## The enforceable rule for every operation, starting now

**Every piece of an operation's tunable state must be reachable
exclusively through `parameters()` / `get_parameter()` / `set_parameter()`
- never a private field mutated some other way (a bespoke setter, a
field written directly from graph-edit code, a "just this once" bypass
because the parameter system felt like overhead for one flag).** This is
already true of every operation in the tree today (see
`operations/transform/resize.rs` for the canonical shape) - this file
exists to make it an explicit, permanent rule rather than an accident of
how things happen to be written so far, since a future keyframe evaluator
has exactly one hook (`set_parameter`) and can only reach state that goes
through it.

This costs nothing to follow - it's the same amount of code either way -
which is why it's decided now instead of left as a convention someone
has to independently rediscover.

## Explicitly not decided yet (deferred, not blocked)

These are real design questions, but don't need answers until keyframing
is actually being built - deciding them now would be guessing, not
saving future work:

- Curve/keyframe data shape (interpolation modes - linear/smooth/step;
  extrapolation before the first and after the last key - hold/linear/
  cycle; whether tangents are user-editable).
- Which `ParameterKind`s are animatable. `Number` is the obvious first
  target (matches Nuke/AE/Fusion, which animate numeric knobs first and
  treat Boolean/Enum/Color/Text as usually-static or step-only). No
  `ParameterKind` shape change is needed either way - the curve lives
  next to the parameter in graph state, not inside `ParameterKind`
  itself.
- Expressions (a knob's value being a formula referencing other knobs/
  time, not just a keyframe curve) - a real Nuke/Fusion feature, strictly
  more work than keyframing alone, and not needed for keyframing itself
  to work.
- UI: curve editor, keyframe add/remove/drag interaction, timeline
  scrubbing UX.
