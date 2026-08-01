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

## MOVE operation

**Work:** 6h · **Complexity:** 5/6
**Depends on:** A general keyboard-input-context system that doesn't
exist yet. MOVE's intended UI (arrow keys nudge the selected node's
position while its EDIT screen is open) collides with keyboard scrubbing
(arrow keys step the focused canvas's video forward/back) - today there
is exactly one global `keydown` listener (Space-bar only, in `app.js`)
with no concept of "what do arrow keys mean right now." Bolting a
MOVE-specific special case onto that would just relocate the conflict,
not solve it. This is a prerequisite design task, not an external
blocker - nothing stops someone from doing it, it just hasn't been done.
**Existing non-functional code:** `engine/src/graphics/geometry.rs`'s
`Point2D`/`Center` structs - defined, unused (`cargo build` flags both as
dead code), scaffolding for the position value MOVE would read/write.
Nothing else exists for this yet - no operation, no UI.

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
