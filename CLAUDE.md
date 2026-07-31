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

## Parked work (postponed on purpose - read before touching these)

### MOVE operation / `graphics/geometry.rs` (`Point2D`, `Center`)

`Point2D`/`Center` exist but are unused - they're scaffolding for a
planned MOVE transform operation that was never implemented. Postponed
specifically because MOVE's intended UI (arrow keys nudge the selected
node's position while its EDIT screen is open) collides with keyboard
scrubbing (arrow keys step the focused canvas's video forward/back) -
today there is exactly one global `keydown` listener (Space-bar only,
in `app.js`) with no concept of "what do arrow keys mean right now."
Bolting a MOVE-specific special case onto that would just relocate the
conflict, not solve it.

Before starting MOVE (or any other operation that wants its own
keybindings - ROTATE/SCALE are likely next), design a general keyboard-
context system first: something like a stack of "current input context"
that a node's EDIT mode can push (claiming arrow keys) and pop on exit,
falling back to whatever scrub/transport context was underneath. Open
questions to resolve before implementing, from the last discussion:

1. Should scrub-while-editing-MOVE ever work simultaneously, or is it
   always strictly one-or-the-other?
2. What other keys/operations need this same context-switching, besides
   arrow keys for MOVE?
3. Should the context be tied to "a node is in EDIT mode" specifically,
   or more generally to whatever menu/screen is currently open?

### Video playback: two different approaches, only one is live

There are two unconnected ways to get a video frame into the graph:

- **Live (what's actually used today):** hand Rust an `HtmlVideoElement`
  (`set_pixel_source_on_node`); every tick, draw whatever the browser is
  currently showing onto a scratch canvas and read pixels back. The
  browser owns `currentTime`/seeking/play-pause entirely. Keyboard
  scrubbing (accelerating hold-to-scrub, ideally via a sigmoid ramp on
  hold duration - discussed but not built) would extend this: an arrow-
  key handler nudging `videoEl.currentTime` directly. `engine/transport.js`
  already flags that scrub/seek was deliberately deferred - only
  play/stop/rewindToStart exist so far.
- **Dead (`operations/sources/video.rs`'s `VideoSource::set_video`/
  `get_video`, `Value::Video`, `Video::frame_at`):** pre-decode an entire
  video into a `Vec<Arc<Image>>` in memory, then index into it by time.
  Nothing on the JS side ever calls `set_video()`, so this path is inert.
  It exists because it would give frame-exact stepping (a `<video>`
  element only exposes continuous seconds, not discrete frame indices)
  and true reverse playback (browsers don't support that reliably via
  `currentTime`/negative playback rate) - relevant if the live approach's
  scrub ever feels imprecise, or for a future frame-accurate export
  feature, but far heavier (decode + hold the whole video in memory) and
  currently unfinished on both the Rust and JS sides. Don't wire it up
  without deciding it's actually worth that cost over the live path.
