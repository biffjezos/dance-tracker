# Parked work (postponed on purpose - read before touching these)

This file tracks specific, deliberately-postponed implementation work -
tickets, effectively. It's separate from `CLAUDE.md` so that file can stay
focused on standing behavioral rules for anyone (or any agent) working in
this codebase, rather than growing into a backlog.

Every entry below follows the same format - see CLAUDE.md's "Editing
PARKED_WORK.md" section for what each field means and why (in short: so a
session that hasn't seen this file before can tell, at a glance, whether
an item is genuinely blocked or just unstarted, and whether any code
already in the tree belongs to it before touching that code).

## Stale DFS state causes false-positive cycle flags in graph validation

**Work:** 1.5h · **Complexity:** 3/6
**Depends on:** Nothing - unimplemented fix, not blocked. Found while
adding regression tests for a related (and fixed) validation bug; not
fixed itself because it needs its own careful test coverage and this
session was already deep in an unrelated change.
**Existing non-functional code:** None - this is a live bug in
already-shipped code, not a stub.

`compositor/graph/validate.rs`'s `run_validation` declares `state: Vec<VisitState>`
and `path: Vec<NodeId>` once, then reuses both across every top-level DFS
root in its cycle-detection loop. `visit_cycle_detection` only pops `path`
and sets `state[index] = Visited` on its *normal* exit - the early
`return Err(cycle)` taken when a real cycle is found skips both, leaving
every node on that aborted path stuck at `VisitState::Visiting` and still
sitting in `path` for good. The next top-level root processed in the same
`run_validation` call can then walk into one of those stale `Visiting`
entries and misread it as a fresh cycle - even though the node it's
looking at isn't actually part of any cycle - because `state[..]` says
"currently being visited" and `path` (still holding leftover entries from
the earlier, already-aborted traversal) happens to contain that index too.

Net effect: in a graph with a real cycle somewhere, some unrelated node
elsewhere in the graph can be incorrectly flagged `NodeValidation::Cycle`
too, depending on node creation/traversal order - a red "Part of a wiring
cycle" badge on a node that was never wired anywhere near the actual
cycle. Likely fix: reset `state`/`path` per top-level root (or scope them
to `visit_cycle_detection`'s own call), and pop/reset on the early-return
path too, not just the normal one. Needs a regression test exercising
exactly this multi-root scenario (a cycle plus an unrelated node visited
afterward) before landing.

## MOVE operation: arrow-key nudging

**Work:** 6h · **Complexity:** 5/6
**Depends on:** A general keyboard-input-context system that doesn't
exist yet. The base MOVE operation itself now exists
(`engine/src/operations/transform/move_op.rs`) - `OFFSET_X`/`OFFSET_Y`
are plain `Number` parameters, edited through the existing generic
stepper UI (the same steppers RESIZE's `SCALE_X`/`SCALE_Y` use). What
remains parked is only the originally-envisioned *additional* UI: arrow
keys nudging the selected node's position while its EDIT screen is open.
That collides with keyboard scrubbing (arrow keys step the focused
canvas's video forward/back) - today there is exactly one global
`keydown` listener (Space-bar only, in `app.js`) with no concept of
"what do arrow keys mean right now." Bolting a MOVE-specific special
case onto that would just relocate the conflict, not solve it. This is
a prerequisite design task, not an external blocker - nothing stops
someone from doing it, it just hasn't been done.
**Existing non-functional code:** `engine/src/graphics/geometry.rs`'s
`Point2D`/`Center` structs - defined, unused (`cargo build` flags both as
dead code), predate MOVE and were deliberately not used by it (MOVE
stores `offset_x`/`offset_y` as plain `f64` fields, matching RESIZE's
`scale_x`/`scale_y` - see the MOVE spec's "Out of scope" section). Still
nothing claims them; leave them for whoever eventually needs a
`Point2D`-shaped parameter type.

Before adding arrow-key nudging to MOVE (or any other operation that
wants its own keybindings - ROTATE/SCALE are likely next), design a
general keyboard-context system first: something like a stack of
"current input context" that a node's EDIT mode can push (claiming
arrow keys) and pop on exit, falling back to whatever scrub/transport
context was underneath. Open questions to resolve before implementing,
from the last discussion:

1. Should scrub-while-editing-MOVE ever work simultaneously, or is it
   always strictly one-or-the-other?
2. What other keys/operations need this same context-switching, besides
   arrow keys for MOVE?
3. Should the context be tied to "a node is in EDIT mode" specifically,
   or more generally to whatever menu/screen is currently open?

## Frame-accurate video decode (ProRes)

**Work:** 16h · **Complexity:** 6/6 (Opus territory - real codec/WASM
integration work, not a Sonnet-default-effort task)
**Depends on:** Nothing blocking - the format decision and WASM
feasibility check are both already done (see below). The one open item
is validating the decoder against real-world files before depending on
it in production.
**Existing non-functional code:** `operations/sources/video.rs`'s
`VideoSource::set_video`/`get_video`, `compositor/value.rs`'s
`Value::Video`, `graphics/video.rs`'s `Video::frame_at` - all defined,
none ever called from the JS side (`set_video()` has no caller anywhere),
so this whole path is currently inert. This is the "pre-decode into
memory" side of the two-approaches split described below, not
`oxideav-prores` integration itself, which hasn't been started.

There are two unconnected ways to get a video frame into the graph:

- **Live (what's actually used today):** hand Rust an `HtmlVideoElement`
  (`set_pixel_source_on_node`); every tick, draw whatever the browser is
  currently showing onto a scratch canvas and read pixels back. The
  browser owns `currentTime`/seeking/play-pause entirely.
- **Dead (the scaffolding listed above):** pre-decode an entire video
  into a `Vec<Arc<Image>>` in memory, then index into it by time. It
  exists because it would give frame-exact stepping (a `<video>` element
  only exposes continuous seconds, not discrete frame indices) and true
  reverse playback (browsers don't support that reliably via
  `currentTime`/negative playback rate) - relevant if the live approach's
  scrub ever feels imprecise, or for a future frame-accurate export
  feature, but far heavier (decode + hold the whole video in memory) and
  currently unfinished on both the Rust and JS sides.

**Decide the codec in Rust rather than relying on the browser** - browsers
don't decode any professional intermediate codec (ProRes/DNxHD/CineForm)
natively, and this app has no server to transcode uploads first. All-intra
codecs (frame independent, no inter-frame prediction) are what make
random-access frame decode cheap regardless of position in an hour-long
file - that's the actual property being bought here, not "Rust decode"
per se.

Decided: **ProRes**, via `OxideAV/oxideav-prores` (pure Rust, MIT,
decode+encode, all 6 profiles, 8/10/12/16-bit) - over DNxHD/CineForm,
which have no pure-Rust implementation and would mean wrapping `ffmpeg`
(DNxHD) or GoPro's open-sourced reference C SDK (CineForm) via FFI.
Pure-Rust isn't a hard requirement, but is preferred when two options are
otherwise equal - and ProRes is the more prevalent format in real footage
today anyway, so it wins either way. Sanity-check `oxideav-prores`
against real camera/NLE-exported ProRes files before depending on it -
its own claims (fuzzed, benched, beats reference encoders on PSNR) are a
good maturity signal but aren't the same as passing an independent
conformance suite.

(Ruled out separately: `OxideAV/oxideav-h266`, a pure-Rust H.266/VVC
decoder from the same org - looked promising from its README, but its
own conformance tests pass 0/56 official JVET streams, and H.266 has
near-zero real-world adoption to decode anyway. Not revisited unless
that changes substantially.)

WASM feasibility check, passed:
- `oxideav-prores` builds clean for `wasm32-unknown-unknown` (~44KB release)
- `oxideav-mov` (QTFF demuxer) builds clean too, off `Cursor<Vec<u8>>` (~336KB combined)
- native roundtrip (encode→decode, several profiles/bit depths) via the
  crate's own example: correct

Still open: never tested against a real camera/NLE-exported ProRes file
(only the crate's own encoder output and its own conformance claims).

## Frame-exact transport controls (play/stop/scrub)

**Work:** 6h · **Complexity:** 3/6 (mostly UI wiring once the harder
frame-decode problem is solved by the item above)
**Depends on:** "Frame-accurate video decode (ProRes)", directly above.
Real scrubbing/frame-stepping needs discrete frame indexing, which the
live `HTMLVideoElement` path (what exists today) can't provide reliably -
`<video>` only exposes continuous seconds via `currentTime`, not frame
numbers, and reverse playback isn't reliable via a negative playback
rate. This item should not be started before that one lands; there is no
separate frame-exact-transport-only shortcut.
**Existing non-functional code:** None beyond what the dependency above
already lists. `ui/scripts/engine/transport.js` currently exports
working (not stub) `play`/`stop`/`rewindToStart` against a live
`HTMLVideoElement` - deliberately minimal on purpose, per its own top
comment, pending this work. Extend it, don't replace it.

Full transport UI - play, stop, and frame-accurate scrubbing (accelerating
hold-to-scrub, ideally via a sigmoid ramp on hold duration - discussed but
not designed in detail) - once the ProRes decode path above lands and
frames can be addressed by index instead of only by continuous seconds.

## Deeper menu navigation via OperationCategory

**Work:** 4h · **Complexity:** 3/6
**Depends on:** A taxonomy decision, not a technical blocker.
`OperationCategory` (`compositor/metadata.rs`) currently has 6 broad
buckets - Source, Generator, Mask, Composite, Reference, Color - and
every TRANSFORM-menu operation (blur, invert, resize, rgb_to_hsv,
shuffle) is tagged `Color` today, with no distinction between "colour
manipulation" (invert, shuffle), "colorspace conversion feeding a keying
op" (rgb_to_hsv - wired as hue_key's Reference input), and "pixel/
spatial manipulation" (blur, resize). `Reference` is defined but not
used by any operation yet. Someone needs to decide the actual category
split - and whether it's even the right axis (vs. e.g. splitting
TRANSFORM into more top-level menus instead) - before there's anything
meaningful for a submenu to group by. This will matter more once
Constellation/Rings/Ghost/Text land and GENERATE stops being small.
**Existing non-functional code:** None dead, but under-used, not a
stub: `OperationMetadata.category` is already correctly populated per
operation, and both `OperationRegistry::describe_all()`
(`compositor/registry.rs`) and `App::get_operations()` (`app.rs`)
already serialize it to JS as `category` on every operation view - this
part is real, working, already-shipped plumbing, not scaffolding. It's
simply never read on the JS side yet: `menu.js`'s `renderOperationList()`
only ever filters by the flat `menu` string, so `category` arrives in
every `get_operations()` response today and is currently ignored.

Once the taxonomy above is settled, the JS-side work itself is small:
group `renderOperationList()`'s filtered operations by `category` within
a menu and push a second-level submenu context (the node-selector/
param-group push/pop pattern already in `menu.js`/`nodeEditContexts.js`
is directly reusable) instead of one flat button list per menu.

## MORPH operation (SDF-based mask shape interpolation)

**Work:** 6h · **Complexity:** 4/6
**Depends on:** Nothing blocking - the prerequisite plumbing this was
originally scoped after (typed input compatibility, `InputDescriptor`/
`accepts` on `OperationMetadata.inputs`) is done, merged into `dev` via
PR #97. Two design questions are open, not blocking, but worth resolving
before starting:

1. Which two `Input` slots carry "mask state A" and "mask state B" - the
   current `Input` enum (`Source`/`Reference`/`Content`/`Mask`/
   `Foreground`/`Background`) has no natural pair for two same-role mask
   inputs. Reusing `Foreground`/`Background` is the obvious fit (already
   typed to accept pixel data via `PIXEL_KINDS`), but overloads names
   otherwise associated with the `compose` operations' blend semantics.
2. Whether progress over the `DURATION` (frames) parameter is driven by
   an explicit start-frame/trigger or read directly off `ctx.meta.frame`.
   This is the same category of "how does a built-in-animated operation
   know when to start counting" question RING's PULSATE apparently ran
   into (added in `b5e2b07`, reverted in `eee1113`, no reason recorded in
   either commit message) - worth understanding why PULSATE was reverted
   before picking the same time-driving approach here.

**Existing non-functional code:** None. `compose/mix.rs` is the nearest
existing pattern (two-pixel-input blend) but performs a flat alpha
crossfade, not a shape morph, and was explicitly ruled out as
insufficient for this feature: a naive alpha crossfade of a circle mask
and a box mask ghosts/double-exposes mid-transition (both shapes visible,
fading) rather than producing a single continuously-deforming solid
silhouette.

A MORPH operation that takes two masks (state A, state B) and a
frame-count `DURATION`, and outputs a mask whose silhouette continuously
deforms from A's shape to B's over that duration - via signed-distance-
field interpolation (distance-transform both source masks, linearly
interpolate the two distance fields by frame progress, threshold back to
a solid alpha mask each frame), not a per-pixel alpha crossfade. The
distance transform is real per-pixel compute, more than anything
currently in `compose`/`key`, though still workable at this app's
resolutions.

Motivating example: a filled RING (THICKNESS at its max, i.e. a solid
disc) morphing into a small box (a resized CHECKERBOARD, same colours) -
wired as another node's MASK, the masked footage's visible shape animates
circle -> box as the underlying mask morphs.

## Content-derived bbox tightening for chromakey/hue_key

**Work:** 5h · **Complexity:** 4/6
**Depends on:** The base bounding-box mechanism (`BBOX_CONVENTIONS.md`,
`Rect`/`Operation::output_bbox`/`Context.input_bboxes`) landing first -
this round's implementation spec deliberately scopes that mechanism to
`RESIZE`/`MOVE`/`BLUR`/geometric masks only, explicitly excluding this
item. Management's call: chromakey is the headline case for why bbox
awareness matters (a green-screen key's silhouette is exactly the kind of
"most of the frame doesn't matter" region this whole effort is about), but
its region has no fixed shape and can change every frame on video - real
content-dependent work, not a parameter-derived box like `BLUR`'s
kernel-radius grow. Validate the mechanism on the simpler, static/
geometric cases first, then come back to this.
**Existing non-functional code:** None yet - `output_bbox()`'s trait
signature already includes an `output: &Value` argument specifically so
this item doesn't require a second signature change when it's picked up
(see `BBOX_CONVENTIONS.md`'s "where this lives in the type system"
section).

Once the base mechanism exists: add a shared helper (`graphics/bbox.rs` or
alongside `apply_mask` in `graphics/mask.rs`) that scans a computed
alpha channel for the tightest enclosing `Rect` where `alpha > 0.0`
(strict, not a rounding threshold - a barely-nonzero pixel could still
matter to something downstream, e.g. an ADD). `chromakey`/`hue_key`
override `output_bbox()` to call it on their own `output` argument. Open
question worth resolving before starting: since a keyed region can change
shape every frame on video, is per-tick re-scanning (a single O(width *
height) linear pass, cheap relative to the keying math itself) sufficient,
or does the box need any temporal smoothing/hysteresis to avoid a
flickering box size destabilizing downstream ops' own compute-region
choices tick to tick? No evidence either way yet - worth a real
measurement before deciding, not a guess.

## Backfill ADRs for architectural decisions predating the ADR artifact type

**Work:** 3h · **Complexity:** 2/6
**Depends on:** Nothing - unimplemented, not blocked. Purely a documentation
backfill; every decision listed below is already made, already implemented,
and already recorded somewhere (a convention file or a specification) -
this is about giving each one its own traceable `ADR-ID` per
`communication_protocol.md`'s Architecture Decision Record format
(Context/Decision/Alternatives considered/Consequences/Technical impact/
Related specifications), not re-deciding anything.
**Existing non-functional code:** None - this is a documentation gap, not
a code gap. The decisions themselves are real and shipped; only the
formal ADR record is missing.

Candidate decisions to backfill, identified while syncing working state
(none of these existed as ADRs because the ADR artifact type didn't exist
in the workflow yet when they were made):

1. **Bounding-box report-vs-consume split, and `Rect` as a single
   axis-aligned rectangle** (`BBOX_CONVENTIONS.md`) - arguably the most
   significant architectural decision of the bbox work; currently only
   documented as a convention, not as a decision record with alternatives
   considered.
2. **`Context` gains a field (`input_bboxes`, later `gpu`) rather than
   `execute()` gaining a new parameter** (`BBOX_CONVENTIONS.md`,
   `SPEC-webgpu-compute-backend.md`) - a real tradeoff with a stated
   alternative that was rejected (a new `execute()` parameter, rejected
   for the test-fixture churn it would have caused) - a natural ADR
   candidate since "alternatives considered" is exactly this.
3. **One-tick-latency pipelined GPU dispatch instead of blocking or a
   fully async execution engine** (`SPEC-webgpu-compute-backend.md`) - the
   core answer to "how does synchronous `execute()` use an inherently
   async WebGPU readback." Two real alternatives were considered and
   rejected (blocking the browser thread - impossible; making the whole
   engine async - out of proportion) - another clean ADR candidate.
4. **Typed input compatibility (`InputDescriptor`/`accepts`, empty list =
   unrestricted)** - shipped (PR #97) but never had a standing convention
   file *or* an ADR; currently the least-documented of the four despite
   being real, load-bearing architecture (it's what the menu-consolidation
   spec's registration design assumes exists).

Not urgent - these are stable, already-implemented decisions, not
open questions - but worth doing before institutional memory of *why*
each one was made (not just *what* was decided) fades further from
convention-file prose into something only reconstructable by reading
old specs/chat history.

## Shrink pixel buffers to their bounding box (real RAM reduction)

**Work:** 25h · **Complexity:** 6/6 (Opus territory - a fundamental
change to the engine's pixel representation, not a Sonnet-default-effort
task)
**Depends on:** The current bounding-box mechanism (`BBOX_CONVENTIONS.md`)
landing in full first - ideally including "Content-derived bbox
tightening for chromakey/hue_key" above, not just the geometric/compute
round. This item reuses the exact same `Rect` data every operation
already reports; it does not need a separate box-tracking system. What
it needs is time for that `Rect` data to be proven correct across real
usage first, because this item raises the stakes of a wrong box
considerably. Today (compute-only), an incorrectly-too-small reported box
means an operation skips computing some pixels it shouldn't have -
visibly wrong output, but a safe, bounded failure (the buffer still has
room for every pixel; only the wrong ones are stale). Once buffers are
actually sized to the box, the same bug means the buffer has **no memory
allocated** for the pixels outside it at all - an out-of-bounds write/read
class of bug, not just a wrong-pixel one. Do not start this until the
compute-only mechanism has shipped across every operation category and
held up in practice.
**Existing non-functional code:** None - `BBOX_CONVENTIONS.md`'s own
"Out of scope" section explicitly excludes this from the current round;
nothing in the tree attempts it.

**Why this is a genuinely bigger change than the current bbox work, not
just "the same thing but for allocation instead of loop bounds":** every
pixel-bearing type (`FloatImage`, `U8Image`, `Frame`, `Mask`) is
currently always exactly `ctx.meta.width x ctx.meta.height`, implicitly
anchored at `(0,0)` - every operation's pixel-index math
(`(y * width + x) * 4`) and `apply_mask`'s dimension-match check assume
this everywhere, not just in the handful of operations touched so far.
Making a buffer's actual size and origin match its reported `Rect`
instead means:

- `FloatImage`/`U8Image`/`Frame`/`Mask` gain an offset (`x0`, `y0`) and
  their `width`/`height` become the *local* buffer's own size, not the
  frame's - a real struct shape change, not an additive field.
- Every operation's own pixel loop needs to work in local-buffer
  coordinates while still correctly reading neighboring pixels from
  other inputs that may have a *different* size and origin (two masked
  inputs to a COMPOSE operation, e.g. `ADD`'s `Foreground`/`Background`,
  can easily have different boxes - combining them correctly means
  reasoning in absolute frame coordinates while writing to/from
  differently-offset local buffers).
- `apply_mask` stops being "blend two same-size buffers" and becomes
  "composite a smaller buffer onto a larger one at an offset" - a real
  rewrite of the one shared mechanism every masked operation depends on.
- The WASM/JS boundary and final canvas draw need to place a
  variable-sized, offset output at the correct absolute position on a
  fixed-size browser canvas - today every buffer that reaches the canvas
  is already full-size and drops in directly.

This is why `BBOX_CONVENTIONS.md` scoped the current round to compute
only: the RAM win is real and matches your camera/HD-video example
below, but it's a different, larger engineering effort than "add a bbox
mechanism," not a follow-on phase of it.

**Motivating example (from conversation):** load an HD video, key it
(`chromakey`/`hue_key`), blur the result (masked by the key's own mask),
run it through several more operations, then mask it down to a solid
shape or a hard crop. Today, and even after the full compute-only bbox
mechanism above ships, every one of those intermediate buffers - the
decoded HD frame, the key's mask, the blur's output, everything after it
- stays allocated at full HD resolution regardless of how aggressively
the final crop/mask restricts what's actually visible. Only this item
would let a pipeline like that actually hold less memory, proportional to
how much of the frame survives to the end, rather than proportional to
the source resolution alone.

## Constellation generator effect

**Work:** 3h · **Complexity:** 2/6
**Depends on:** Nothing - unimplemented, not blocked. A straightforward
`Generator`-category operation, same shape as the existing `checkerboard`/
rings-style generators already in the codebase to use as reference.
**Existing non-functional code:** None currently in the tree. Its old
(dead, deleted) UI hook was `ui/scripts/features/notWired.js`'s
`toggleConstellation`/`constellationDistanceUp`/`constellationDistanceDown`
`window` event stubs - removed, since nothing dispatched them and the
current architecture doesn't work that way (see below).

A star-field/particle generator effect with a toggle and a distance/depth
parameter, from an earlier, pre-node-graph version of the app.

## Rings generator effect

**Work:** 3h · **Complexity:** 2/6
**Depends on:** Nothing - unimplemented, not blocked.
**Existing non-functional code:** None currently in the tree - no Rust
operation, no UI. `ui/assets/rings.png`, `key-rings.png`, `key-rings-2.png`
are screenshots of it existing in some earlier build, not code. Its old
(dead, deleted) UI hook was `notWired.js`'s `toggleRingsEnabled` stub.

A rings generator effect with an enable/disable toggle, from the same
earlier version of the app as Constellation above.

## Audio sync

**Work:** 10h · **Complexity:** 5/6
**Depends on:** No audio input/analysis operation of any kind exists in
the graph today - there's no way to get audio into a node chain at all
yet. That's a prerequisite design task (what does an audio source/
analysis operation even look like here), not just a matter of adding
parameters to an existing operation.
**Existing non-functional code:** None currently in the tree. Its old
(dead, deleted) UI hooks were `notWired.js`'s `audioSyncMinuteUp/Down`,
`audioSyncSecondUp/Down`, `audioSyncFrameUp/Down` stubs.

Step an audio-sync offset by minute/second/frame, for aligning generated
visuals to an audio track. Once an audio source exists, this would likely
become a grouped `Number` parameter set (see `group` on
`ParameterDescriptor`) on whatever operation ends up owning audio sync.

---

For all three effects above: per "no default anything" (CLAUDE.md),
these only become real once someone actually implements and wires them
as proper operations - not by reintroducing bare `window` event stubs.
