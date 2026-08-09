RFC-ID: RFC-008
Created: 2026-08-09
Created-By: software_architect
Target-Role: software_developer
Related-Specification: SPEC-OUTPUT-RENDER-V1 (`.agents/roles/software_architect/docs/specifications/SPECoutputrenderv1.md`)
Priority: medium
Status: Open

Severity: N/A (new feature, not a bug fix)
Finding: N/A
Evidence: N/A
Required Change:

Implement SPEC-OUTPUT-RENDER-V1 — read it first, it has the full
grounding (confirmed by direct source inspection: the OUTPUT menu
category, the `nodeEditContextRegistry` bespoke-context pattern, and the
existing-but-unreachable `Recorder`/`toggleRecord` mechanism this reuses)
and exact acceptance criteria. Summary:

1. New `OperationCategory::Output` variant in
   `engine/src/compositor/metadata.rs`.
2. New `Render` operation in a new `engine/src/operations/output/`
   module (`id: "render"`, `menu: "OUTPUT"`, `label: "RENDER"`,
   `create_node: Some("render")`, zero inputs, zero parameters, trivial
   `execute()` stub) — register the new module in
   `engine/src/operations/mod.rs`.
3. New bespoke edit-context registration for node kind `"render"` in
   `ui/scripts/engine/nodeEditContexts.js`
   (`nodeEditContextRegistry.register("render", ...)`, mirroring the
   existing `renderImageEditContext` precedent): calls
   `renderGenericEditContext` first (gives the empty EDIT screen for
   free, since RENDER has zero inputs/params), then appends one
   `EXECUTE` button that dispatches
   `window.dispatchEvent(new CustomEvent("toggleRecord"))` — the exact
   event `output.js`'s existing (currently unreachable) listener already
   handles correctly. No new recording logic anywhere.
4. Remove the `REC` field from `ui/templates/partials/statusbar.hbs` and
   its corresponding status-bar text update in `output.js`'s
   `toggleRecord` listener — the listener's actual start/stop/download
   logic is unchanged, only the now-redundant status text write goes.
   Leave `STATUS`/`FPS`/`TYPE`/resolution untouched.

This is intentionally the smallest possible slice — Management explicitly
deferred FILE_NAME, FORMAT, RESOLUTION, FPS, COMPRESSOR, FROM/TO, and the
broader status-bar redesign to a later round (see the spec's "Out of
scope" section for why each one is deferred, not just that it is).

Testing requirements:

- Standard per-operation test shape for `Render` (descriptor/metadata
  sanity, registration via inventory) matching the existing pattern other
  simple operations use.
- No JS test harness exists in this repo today (confirmed during
  research) — manual verification of the menu/EDIT/EXECUTE flow is
  acceptable for the UI-side changes; note this explicitly in your
  report rather than treating it as skipped silently.
- Per established precedent (RFC-005/006/007): if `cargo build`/`cargo
  test` are blocked by sandbox network policy, record that explicitly as
  unverified in your Implementation Report.

Acceptance Condition: Code Reviewer approval per the Implementation
Review Loop, then Management approval.
