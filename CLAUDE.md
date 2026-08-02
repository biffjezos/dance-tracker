# Dance Tracker 5000

Realtime demo-scene video synthesizer. Backend is a Rust/WASM node graph
(`core/`); `app.js` + `engine/menu.js` translate UI events into calls
against that graph. See `app.js`'s own top comment for the current
scope/state of the rewrite.

## Hard rules (repeatedly violated, repeatedly reported - do not reintroduce)

### 1. No default anything - ever

Never create a node, operation, mask, or any other graph state that the
user did not explicitly ask for, at any time, including app start.

- No default nodes on boot. The graph starts **empty**. Nothing renders
  until the user explicitly adds something (INPUT, GENERATE, KEY).
- No default masks. A mask exists only after the user clicks ADD MASK.
  Never conjure a "MASK 1" (or any node) because a video happened to
  load, or because some other unrelated setting changed.
- This applies to UI state too, not just graph nodes: never initialize
  a selection/stepper with a hardcoded label like `"MASK 1"` or
  `"VIDEO 1"` as a placeholder. If nothing real exists yet, the UI must
  say so plainly (e.g. "NO MASKS YET") - never show a fake entry that
  *looks* like it's backed by a real node when it isn't.

### 2. Only display what actually exists

Any stepper, list, or picker must be bounded by the real count of real
things, never a hardcoded maximum. If a RINGS node has 2 groups, its
colour-stroke picker offers exactly 2 - never a fixed "up to 8" range
regardless of how many actually exist. If a registry is empty, say so;
don't show slot 1 of a fixed-size range that doesn't correspond to
anything real.

### 3. Stacking happens only through an explicit BACKGROUND wire

This is node-based, not layer-based. There is no implicit z-order, no
"everything currently visible gets merged," no auto-compositing of
independently-existing things. If there's no explicit BACKGROUND
(a user-created Compose wire) between two things, do not lay one over
the other - not by default, not as a fallback, not because both happen
to be "enabled" at once.

- No MOVE UP/DOWN, no manual layer ordering UI. The only way two things
  end up composited together in the output is the user explicitly
  setting one's BACKGROUND to the other.
- The master OUTPUT reflects the currently selected node's own explicit
  wiring (its own MASKED BY / BACKGROUND, if the user set them) - never
  an automatic merge with whatever else happens to exist or be enabled.

### 4. Don't over-invest in throwaway browser test scripts

Prefer Rust unit tests (`cargo test`) for verifying operation/graph
logic - they're fast, precise, and cheap to run on every change. Use
Playwright only as a diagnostic tool when something can't be verified
at the Rust level (actual DOM/canvas/browser behaviour), and don't
commit elaborate one-off test suites as project deliverables. A passing
test suite is not the goal - a working app is. If the app is broken,
say so, don't paper over it with more tests.

## Core pixel & color conventions

How pixel data is tagged (colorspace) and blended (alpha handling) across
every operation is a deliberate, already-decided convention, not
something to re-derive - or "fix" - per operation. Read
`PIXEL_CONVENTIONS.md` before adding a new pixel-producing or
COMPOSE-menu operation, and before treating the current uniform-channel
alpha blending in `add`/`multiply`/`screen` as a bug.

## Animation (keyframing) convention

No keyframing exists yet, but the one rule that keeps it retrofit-free
when it's eventually built is already in force: every operation's
tunable state must go through `parameters()`/`get_parameter()`/
`set_parameter()`, with no exceptions. Read `ANIMATION_CONVENTIONS.md`
before adding a new parameter, or before assuming a future animation
system would need `Operation`/`ParameterKind` to change shape.

## Bounding-box (bbox) awareness convention

Not implemented yet, but the shape of the mechanism is already decided so
it doesn't get retrofitted per operation later: every operation reports a
bounding box for its own output (defaulted to full-frame, always safe),
and may separately, optionally, consume its inputs' boxes to skip real
per-pixel work outside the relevant region. Read `BBOX_CONVENTIONS.md`
before adding a new pixel-producing operation's spatial behavior, or
before assuming a node must always compute over the full frame.


## Postponed/backlog work

Specific features that have been deliberately postponed (MOVE operation,
frame-accurate video decode) are tracked in `PARKED_WORK.md`, not here -
read it before touching either of those areas.

### Editing PARKED_WORK.md

Only relevant if you're adding to or changing that file. Every entry
needs, at minimum:

- A title - the deferred feature/fix.
- `**Work:**` / `**Complexity:**` - same `(Xh, Y/6)` scale as an audit
  finding (see "Codebase audit tasks" below).
- `**Depends on:**` - free text: what has to happen first, and why. Say
  "Nothing - unimplemented, not blocked" if nothing actually blocks it
  (most of these are just unstarted, not stuck). This is what lets a
  session - or a direct question from the user - get a real answer
  ("it's parked because X, not because no one's gotten to it") instead
  of having to reconstruct the reasoning from scratch or guess.
- `**Existing non-functional code:**` - free text: any stub/dead code
  already in the tree that belongs to this item (file + why it's inert),
  or "None". This exists specifically so a session doesn't reflexively
  delete scaffolding it doesn't recognize, or re-add a stub that's
  already there under a different name - cross-session, nobody but this
  file remembers which half-finished code belongs to a planned feature
  versus which is genuinely stale.

## Codebase audit tasks

When asked to audit this codebase (quality, maintainability, bug count,
dead code, etc.) rather than to implement something, report first - don't
fix anything as part of the audit unless separately asked to. Cover:

- Overall quality and ease of maintenance (Rust engine vs. JS UI can
  differ - say so if they do).
- How many files must be touched to add a new operation, and separately,
  a new menu/menu item - this changes as the architecture evolves, so
  re-derive it from the current code rather than assuming a past answer
  still holds.
- How many app-breaking or silent bugs can be identified.
- How much dead/unused code, or scaffolding for a planned-but-unbuilt
  feature, exists - including stale documentation that no longer matches
  the code (a guide describing a removed API is as much a liability as
  dead code, since it actively misleads whoever follows it next).

Then categorize every finding into **MUST / SHOULD / COULD / NICE TO
HAVE**, prioritized within each group (most important first, not just
listed in the order found). For each finding, give an effort estimate as
`(work Xh, complexity Y/6)`:

- **work** - realistic engineering hours to fix, as a plain number (e.g.
  `0.25h`, `1.5h`), not a percentage or ratio.
- **complexity** - 1 to 6, how much thinking effort the fix takes for
  Claude Sonnet at this repo's usual working effort. A rating above 6
  means the task is dense/high-stakes enough to warrant switching to
  Opus rather than attempting it at Sonnet's default effort.
