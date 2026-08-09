<!-- Handoff snapshot, version 1, delivered 2026-08-09. Canonical/owned copy:
     .agents/roles/software_architect/docs/specifications/SPECoutputrenderv1.md
     (Software Architect). Read that file for future updates - this copy is
     the point-in-time delivery instance for this handoff, not a second
     source of truth to maintain independently. -->

SPEC-ID: SPEC-OUTPUT-RENDER-V1
Created: 2026-08-09
Created-By: software_architect
Target-Role: software_developer
Status: Open

Title: OUTPUT menu + RENDER node, v1 — EDIT (empty) + EXECUTE, reactivating the existing Recorder

Purpose:

Management wants a RENDER operation under a new OUTPUT menu, with an
EDIT screen and an EXECUTE button, explicitly scoped to the bare minimum
for this round ("give it an empty EDIT button and an EXECUTE Button. No
other settings for now"). Everything else discussed (FILE_NAME, FORMAT,
RESOLUTION, FPS, COMPRESSOR, FROM/TO, and the related status-bar
redesign) is deferred — tracked in "Out of scope" below, not designed
here.

Grounding (confirmed by direct source inspection before writing this):

- `"OUTPUT"` is **already** a reserved category in `ui/scripts/engine/menu.js`'s
  `CATEGORY_ORDER` (line 36) — it renders no button today only because
  nothing is registered under it. The whole menu system is data-driven off
  `OperationDescriptor.menu` (`renderCategoryButtons`/`renderOperationList`,
  `menu.js`) — **no menu.js change is needed** to make an OUTPUT button
  appear; registering an operation with `menu: "OUTPUT"` is sufficient.
- A working recording capability **already exists and already works**:
  `ui/scripts/engine/recorder.js`'s `Recorder` class
  (`canvas.captureStream(60)` → `MediaRecorder`, preferring `video/mp4`,
  falling back to `video/webm`) and `ui/scripts/features/output.js`'s
  `window.addEventListener("toggleRecord", ...)` (lines 98-115), which
  starts/stops it against `document.getElementById("master-layer")` and
  triggers a browser download on stop. **Nothing in the entire `ui/`
  tree currently dispatches `"toggleRecord"`** — confirmed via search —
  so this capability is real but completely unreachable today. This is
  "the old render operation" / "the things we have" Management referred
  to. This spec's job is to give it a reachable trigger, not to rebuild
  recording.
- Per-node EDIT screens are chosen by node kind via
  `nodeEditContextRegistry` (`ui/scripts/engine/nodeEditContexts.js`,
  `NodeEditContextRegistry.renderContext`/`registerDefault`/`register`) —
  a node kind with no bespoke registration falls through to
  `renderGenericEditContext`, which renders wired-input steppers (none,
  for a node with zero declared inputs) then one row per parameter (none,
  for a node with zero declared parameters) — **an empty EDIT screen
  (just the standard node-nav header) already falls out for free from
  declaring zero inputs and zero parameters.** No new "empty EDIT" UI
  work is needed.
- `OperationCategory` (`engine/src/compositor/metadata.rs`, lines 85-95)
  has no variant fitting an operation that produces no pixel/number
  value and exists purely for its side effect — every existing value
  (`Source`, `Generator`, `Mask`, `Composite`, `Reference`, `Color`,
  `Animation`) implies producing something. A new variant is required.
- `engine/src/operations/mod.rs` lists sibling modules
  (`animate`/`compose`/`generators`/`key`/`sources`/`transform`), each
  registered purely via `inventory::submit!` inside its own operation
  file — `engine/src/operations/register.rs` just calls
  `registry.register_from_inventory()`, no per-module wiring needed
  beyond adding `pub mod output;` to `mod.rs`.

Scope:

1. **New `OperationCategory::Output` variant** (`engine/src/compositor/metadata.rs`)
   — add it alongside the existing seven, with `as_str() => "output"`,
   matching the existing pattern exactly. This is the only change to that
   file.
2. **New `engine/src/operations/output/` module**, one new operation:
   `Render` (`render.rs`), following the exact shape every other
   operation already follows (`Operation` trait impl, `inventory::submit!`
   at the bottom, `#[cfg(test)] mod tests` with at least the standard
   `descriptor()`/`metadata()`/registration-shape tests every other
   operation has).
   - `OperationDescriptor`: `id: "render"`, `menu: "OUTPUT"`,
     `label: "RENDER"`, `action: None`, `ui_action: None`,
     `create_node: Some("render")`, `submenu: None`.
   - `OperationMetadata`: `category: OperationCategory::Output`,
     `inputs: vec![]` (zero — v1 has no wired input; the actual capture
     target is the OUTPUT/master canvas itself, not graph-wired content —
     see "Design notes" below), `outputs: vec![OutputKind::Number]` (or
     whatever placeholder `OutputKind` least implies a real pixel/number
     result is expected downstream — nothing will ever consume RENDER's
     output; pick whatever satisfies the trait with the least ceremony).
   - `parameters()`: `vec![]` — explicitly zero for this round.
   - `execute()`: a trivial, harmless stub (e.g. returns
     `Ok(vec![Value::Boolean(true)])` or similar) — RENDER's real behavior
     (starting/stopping the browser recorder) is a JS-side side effect
     triggered by clicking EXECUTE in its EDIT screen, not something the
     graph's compute model produces. `execute()` exists only to satisfy
     the `Operation` trait so RENDER can participate in the graph/menu
     system like every other node.
   - Add `pub mod output;` to `engine/src/operations/mod.rs`, matching
     the existing sibling-module list.
3. **New bespoke node-edit-context registration for kind `"render"`**
   (`ui/scripts/engine/nodeEditContexts.js`), registered via
   `nodeEditContextRegistry.register("render", ...)` alongside the
   existing `registerDefault(renderGenericEditContext)` call — mirroring
   `renderImageEditContext`'s existing precedent for a bespoke,
   node-kind-specific context function. This function must:
   - Call `renderGenericEditContext(menuManager, nodeEntry)` first, so
     RENDER gets the exact same node-nav header / "empty because zero
     params" rendering as every other node, for free and consistently.
   - Then append exactly one button labeled `EXECUTE` whose `onclick`
     dispatches `window.dispatchEvent(new CustomEvent("toggleRecord"))` —
     the same event `output.js`'s already-existing listener (line 99)
     already handles correctly (start/stop toggle, download-on-stop).
     **No new recording logic, no new event — EXECUTE's entire job for
     v1 is being a reachable trigger for the event that already exists.**
4. **Status bar: remove the `REC` field.** Its function (a manual
   start/stop toggle) is now reachable via RENDER's EXECUTE button
   instead, per Management's own "REC: obsolete... may be removed" note.
   Remove the `REC: OFF`/`REC: ON` element from
   `ui/templates/partials/statusbar.hbs` and the corresponding
   `bar.children[3]` write in `output.js`'s `toggleRecord` listener
   (lines 103-114) — the listener itself and its start/stop logic stay
   exactly as-is, only the now-redundant status-bar text update goes.
   Do not touch `STATUS`/`FPS`/`TYPE`/resolution/`#gamut-warning` in this
   round — those are separate, larger, explicitly-deferred work (see
   "Out of scope").

Acceptance Criteria:

1. An `OUTPUT` button appears in the menu bar once `Render` is
   registered, with no `menu.js` changes (data-driven, per "Grounding"
   above) — verify this holds, don't special-case it.
2. Clicking `OUTPUT` shows a `RENDER` button; clicking it creates a
   `render` node the same way every other `create_node`-backed operation
   does (via the existing `create_node` submenu flow).
3. Selecting a `render` node's EDIT screen shows only the standard
   node-nav header (`[UP] - RENDER + |` or whatever the existing generic
   header renders) — no parameter rows, no wired-input rows — plus one
   `EXECUTE` button.
4. Clicking `EXECUTE` starts recording the `master-layer` canvas
   (identical behavior to what `toggleRecord` already does today, just
   newly reachable); clicking it again stops recording and triggers the
   existing download-as-file behavior. No regression to `Recorder`'s own
   logic — this spec doesn't modify `recorder.js` at all.
5. The `REC` field no longer appears in the status bar; every other
   status bar field is visually and functionally unchanged.
6. `cargo build`/`cargo test` succeed (existing test suite plus new
   `Render`-operation tests, matching the standard per-operation test
   shape used elsewhere — descriptor/metadata sanity, registration).

Constraints:

- Do not add `FILE_NAME`, `FORMAT`, `RESOLUTION`, `FPS`, `COMPRESSOR`, or
  `FROM`/`TO` in this round — explicitly deferred by Management ("No
  other settings for now").
- Do not add a wired `Input::Source` (or any input) to `Render` in this
  round — v1 captures the OUTPUT/master canvas directly, matching how
  the existing (currently-unreachable) `Recorder` already works; it is
  not graph-aware.
- Do not modify `recorder.js`'s recording logic, `MediaRecorder`
  codec/format negotiation, or the download-filename behavior — all of
  that is explicitly out of scope and already works; this spec only adds
  a trigger for it.
- Do not redesign the status bar beyond removing the `REC` field —
  `STATUS`/`FPS`/`TYPE`/resolution readouts and the eventual
  `[from][current][to]`/`[resolution]` layout are separate, larger,
  future work (see below).

Dependencies: None — every piece this spec touches (menu system,
`Recorder`, `nodeEditContextRegistry`, `OperationCategory`, the
`inventory`-based operation-registration mechanism) already exists and
is already working on `dev`.

Design notes:

**Why zero inputs, not a wired `Input::Source`:** the existing
`Recorder` captures whatever's currently drawn to the `master-layer`
canvas via `canvas.captureStream()` — this is already how the app's
live OUTPUT panel works (`ui/scripts/engine/render.js`'s `loop()` draws
whatever node is set as the LIVE PREVIEW override to that same canvas
every tick). RENDER doesn't need its own wire to "know what to record" —
it records whatever's already being shown, exactly like a physical
camera pointed at a screen would, not like a compositing operation that
needs its own pixel input. A wired wiring model for RENDER (recording a
*specific* node regardless of what the OUTPUT panel currently shows) is
a real, larger design question appropriately deferred alongside
FROM/TO/RESOLUTION, not decided here.

**Known v1 limitation, not fixed here:** clicking EXECUTE only produces
useful output if the OUTPUT panel is actually visible and showing the
intended content at the time (`master-layer` is only redrawn while
`isPanelVisible("output")` is true, per `render.js`'s `loop()`) — RENDER
does not currently verify or enforce this. Acceptable for this round
given the explicit "no other settings" scope; worth a guard/warning in a
later round once FROM/TO exists and EXECUTE means something more
specific than "toggle the existing live capture."

Out of scope (tracked for later, not designed in this document):

- `FILE_NAME` (blocked on `ParameterKind::Text` having no working EDIT-screen
  UI today — `nodeEditContexts.js`'s generic Enum/Text branch requires a
  non-empty `options` list; Text always renders nothing currently).
- `FORMAT`/`COMPRESSOR` (mp4-only for now is already true — `Recorder`
  already prefers `video/mp4` when the browser supports it — a real
  FORMAT/COMPRESSOR *selector* is new UI + possibly new encode-path work).
- `RESOLUTION` (open question: reuse the existing single global
  `set_resolution`, or support an independent export resolution — not
  decided).
- `FPS` (open question: what preset list constitutes "standard
  historic/modern" — not decided).
- `FROM`/`TO` (blocked on this app having no frame-accurate seeking
  today — see `PARKED_WORK.md`'s "Frame-accurate video decode (ProRes)"
  and "Frame-exact transport controls" entries; a time-based-in-seconds
  version is buildable without that work but wasn't decided as
  acceptable for v1 by Management yet).
- The `[from][current][to]` / `[resolution]` status-bar redesign
  (depends on FROM/TO existing first; whether the mockup's symmetric
  left/right triplets mean one per canvas, PREVIEW vs OUTPUT, was never
  confirmed).

These remain open questions for whoever picks up the next round — not
answered here, since Management explicitly scoped this round to
EDIT+EXECUTE only.
