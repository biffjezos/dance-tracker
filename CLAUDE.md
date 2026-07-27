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
