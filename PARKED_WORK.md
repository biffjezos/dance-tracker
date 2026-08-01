# Parked work (postponed on purpose - read before touching these)

This file tracks specific, deliberately-postponed implementation work -
tickets, effectively. It's separate from `CLAUDE.md` so that file can stay
focused on standing behavioral rules for anyone (or any agent) working in
this codebase, rather than growing into a backlog.

## MOVE operation / `graphics/geometry.rs` (`Point2D`, `Center`)

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

## Video playback: two different approaches, only one is live

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

**If/when this gets built:** decode the codec in Rust rather than relying
on the browser - browsers don't decode any professional intermediate
codec (ProRes/DNxHD/CineForm) natively, and this app has no server to
transcode uploads first. All-intra codecs (frame independent, no inter-
frame prediction) are what make random-access frame decode cheap
regardless of position in an hour-long file - that's the actual property
being bought here, not "Rust decode" per se.

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

## Effects that once existed, not yet ported to the current architecture

`ui/scripts/features/notWired.js` used to hold ten `window.addEventListener`
stubs for controls from an earlier, pre-node-graph version of the app. None
of them are dispatched from anywhere in the current UI - nothing wires a
button to these event names anymore, so the file was dead weight and has
been deleted. The event names below are the only surviving record of what
they controlled; they're listed here so the effects themselves aren't lost,
not because the old event-listener approach should be reused.

- `toggleConstellation` - enable/disable a constellation (star-field/
  particle) generator effect.
- `constellationDistanceUp` / `constellationDistanceDown` - step the
  constellation effect's distance/depth parameter.
- `toggleRingsEnabled` - enable/disable a rings generator effect (a rings
  operation exists in screenshots/assets - see `ui/assets/rings.png`,
  `key-rings.png`, `key-rings-2.png` - but is not currently a registered
  Rust operation).
- `audioSyncMinuteUp` / `audioSyncMinuteDown` / `audioSyncSecondUp` /
  `audioSyncSecondDown` / `audioSyncFrameUp` / `audioSyncFrameDown` -
  step an audio-sync offset by minute/second/frame, for aligning generated
  visuals to an audio track.

None of this should be re-wired as bare `window` events again. Per the
current architecture (see the operation-authoring flow the codebase itself
demonstrates - a new `Operation` impl registered via `inventory::submit!`,
picked up automatically by the menu and the generic parameter-edit UI),
each of these should become:

- Constellation and Rings: real `Generator`-category operations in
  `engine/src/operations/generators/`, with their toggle/distance-style
  controls expressed as ordinary `ParameterDescriptor`s (`Boolean` for the
  enable toggle, `Number` with a declared step for distance) - the generic
  edit context already renders steppers for exactly this shape of
  parameter, no bespoke JS needed.
- Audio sync: depends on how audio gets into the graph at all, which
  doesn't exist yet (there's no audio input/analysis operation of any
  kind today). That's a prerequisite piece of design, not just a matter of
  adding parameters to an existing operation - minute/second/frame offset
  would likely become a grouped `Number` parameter set (see `group` on
  `ParameterDescriptor`) on whatever operation ends up owning audio sync.

Not scheduled - listed here so the intent isn't lost, per "no default
anything" this only becomes real once someone actually implements and
wires it, not before.
