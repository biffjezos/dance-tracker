REPORT-ID: REPORT-RFC-008
Created: 2026-08-09
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-OUTPUT-RENDER-V1 / RFC-008
Status: Awaiting Code Review

Summary: Implemented SPEC-OUTPUT-RENDER-V1 as specified — a new OUTPUT
menu category, a `RENDER` operation (`Rust`), and a bespoke EDIT context
with one EXECUTE button that reactivates the existing (previously
unreachable) `Recorder`/`toggleRecord` mechanism. Removed the now-
redundant `REC` status-bar field.

Implemented:

1. `engine/src/compositor/metadata.rs`: added `OperationCategory::Output`
   variant (`as_str() => "output"`), matching the existing pattern
   exactly. Confirmed no other file in `engine/src` exhaustively matches
   `OperationCategory` (grepped for `match` on it — only `as_str()`
   itself does); every other reference is a struct-literal field
   assignment, so no other file needed a matching-arm update.
2. New `engine/src/operations/output/` module
   (`mod.rs` + `render.rs`), registered via `pub mod output;` in
   `engine/src/operations/mod.rs`. `Render`: `id: "render"`,
   `menu: "OUTPUT"`, `label: "RENDER"`, `create_node: Some("render")`,
   zero inputs, zero parameters, `outputs: vec![OutputKind::Boolean]`,
   `execute()` returns `Ok(vec![Value::Boolean(true)])` unconditionally.
   Registered via `inventory::submit!`, same as every other operation.
3. **Spec gap found and resolved, flagging for review:** neither
   RFC-008 nor SPEC-OUTPUT-RENDER-V1 mentions `Operation::supports_edit()`.
   Its default implementation
   (`!parameters().is_empty() || input_count() > 0`) returns `false` for
   an operation with zero inputs and zero parameters — and
   `menu.js`'s `renderEditButton()` gates the entire EDIT button on
   `nodeSelectionState.supportsEdit()`, which round-trips to this exact
   method via `wasmApp.node_supports_edit()`. Left at its default,
   `RENDER` would never show an EDIT button at all, making EXECUTE
   permanently unreachable - directly contradicting RFC-008's own
   Acceptance Criteria 3-4. Overrode `supports_edit()` to return `true`
   unconditionally on `Render` - the trait already provides this as an
   overridable default specifically for operations the heuristic doesn't
   fit (its own doc comment frames it as exactly that), so this is a
   one-method use of an existing, designed-for-this extension point, not
   a new mechanism or an architectural decision. Documented in `Render`'s
   own doc comment and covered by a new test
   (`supports_edit_is_true_despite_zero_inputs_and_parameters`). Judged
   this as within implementation authority (satisfies the RFC's own
   stated acceptance criteria via an existing trait feature) rather than
   something requiring an RFI/architecture stop - flagging here for a
   second opinion given it's a real, if small, gap in the spec's own
   research.
4. `ui/scripts/engine/nodeEditContexts.js`: new
   `renderRenderEditContext(menuManager, nodeEntry)`, registered via
   `nodeEditContextRegistry.register("render", renderRenderEditContext)`.
   Calls `renderGenericEditContext` first (empty screen, since zero
   inputs/params), then appends one `EXECUTE` button dispatching
   `new CustomEvent("toggleRecord")` - the exact event
   `output.js`'s existing listener already handles. No new recording
   logic, no new event, per the spec's own constraint.

   **Correction to the spec's stated precedent:** SPEC-OUTPUT-RENDER-V1
   cites `renderImageEditContext` as an existing precedent for the
   "call generic first, then append" pattern. On inspection,
   `renderImageEditContext` is fully bespoke (doesn't call
   `renderGenericEditContext` at all) and, more importantly, is defined
   but **never actually registered anywhere** in the current tree
   (grepped - `nodeEditContextRegistry.register(` had zero call sites
   before this change) - it's dead/unused code, not a working precedent.
   `.register()`/`renderContext()`'s dispatch logic itself is sound by
   direct reading (a plain `Map` lookup falling back to the default), so
   I used it as specified, but this is the first time this registration
   path is actually exercised in this codebase - noting this since it
   changes how much confidence "matches an existing, working precedent"
   should carry for this specific piece.
5. `ui/templates/partials/statusbar.hbs`: removed the `REC: OFF` span.
   `ui/scripts/features/output.js`'s `toggleRecord` listener: removed
   both `bar.children[3]` REC-text writes (the listener's own
   start/stop logic is untouched).

   **Regression caught and fixed, not mentioned in the spec:** removing
   the `REC` span shifts every later `<span>`'s index by one -
   `output-resolution` moves from `children[4]` to `children[3]`, and
   `applyOutputSize()` (`output.js`) wrote to `bar.children[4]`
   unconditionally by index. Left unfixed, this would have silently
   broken the resolution readout (AC5 requires every other status-bar
   field stay "visually and functionally unchanged"). Updated that one
   index reference to `children[3]`. `status.js`'s own `bar.children[2]`
   (TYPE) is unaffected - REC was after it, not before.
6. `ui/index.html`: manually applied the same `REC` span removal
   `statusbar.hbs` got. This file is a committed, pre-built artifact
   (`ui/build-html.js` regenerates it from `templates/` via
   `npm run build`) - I could not run the actual build (`npm install`
   fails: `registry.npmjs.org` 403, same restriction as
   `NOTIFICATION-002`), so this is a manual, hand-verified-identical
   edit rather than a regenerated file. Flagging so the Code Reviewer
   diffs this file specifically rather than assuming it's build output.

Files modified:

- `engine/src/compositor/metadata.rs`
- `engine/src/operations/mod.rs`
- `engine/src/operations/output/mod.rs` (new)
- `engine/src/operations/output/render.rs` (new)
- `ui/scripts/engine/nodeEditContexts.js`
- `ui/scripts/features/output.js`
- `ui/templates/partials/statusbar.hbs`
- `ui/index.html`

Architecture notes: No new mechanism on either side - `Render` follows
the exact `Operation` trait shape every other operation uses;
`renderRenderEditContext` follows the exact `NodeEditContext`
registration shape the registry already provides (even if not
previously exercised). The one real judgment call
(`supports_edit()` override) is documented above.

Tests executed:

- Rust: none executed (see "Test results" - `cargo` blocked). Wrote
  `render.rs`'s own test module (descriptor/metadata/parameters/
  supports_edit/execute-stub/graph-validity), following `square.rs`'s
  conventions as the closest existing zero-input-operation template
  (no existing "registration via inventory" test convention was found
  anywhere in the codebase to match despite the RFC's phrasing - `submit!`
  registration is exercised structurally by every operation's own
  `inventory::submit!` block, not by a dedicated per-operation test).
- JS: `node --check` on both modified files (`nodeEditContexts.js`,
  `output.js`) - both parse cleanly. No live browser click-through was
  possible: the app needs the compiled WASM module to load at all
  (`wasmApp.node_supports_edit`, `node_parameters`, etc.), and rebuilding
  that requires the same blocked `cargo`/`wasm32` toolchain path already
  documented in `NOTIFICATION-002`. Manually traced the click path
  instead: `menu.js` renders an `OUTPUT` category button once any
  operation reports `menu: "OUTPUT"` (data-driven, confirmed no `menu.js`
  change needed) -> clicking it lists `RENDER` (`create_node: Some("render")`)
  -> creating a node and selecting it -> `nodeSelectionState.supportsEdit()`
  now returns `true` (per the override) -> `renderEditButton()` shows
  EDIT -> clicking it renders `renderRenderEditContext` (registered for
  kind `"render"`) -> empty generic screen + one EXECUTE button ->
  clicking EXECUTE dispatches `toggleRecord`, hitting the existing,
  unmodified listener.

Test results: **Unverified (build/runtime)** - `cargo build`/`cargo
test` blocked (`index.crates.io` 403, per RFC-005/006/007 precedent);
no live browser click-through possible for the reason above. New Rust
tests and the manual click-path trace are the available confidence for
this round, per the RFC's own explicit acceptance of manual JS
verification and the established build-block precedent.

Known limitations:

- No automated or live-browser confirmation of the full menu -> EDIT ->
  EXECUTE -> recording/download flow - see above.
- The two items flagged above (missing `supports_edit()` mention, and
  `renderImageEditContext` being dead code rather than a working
  precedent) are spec-research gaps I resolved myself rather than
  stopping for an RFI, since both are small, don't conflict with any
  stated requirement, and are necessary to satisfy the RFC's own
  Acceptance Criteria. Flagging both explicitly per this project's "do
  not ignore spec conflicts" convention, even though I judged neither
  severe enough to halt implementation.
- `ui/index.html`'s edit is hand-applied, not build-regenerated - see
  item 6 above.

Specification deviations: None from the required behavior/acceptance
criteria. The `supports_edit()` override and the `index.html` hand-edit
are both implementation necessities to satisfy stated ACs, not scope
changes.

Reviewer notes: Please double-check the `supports_edit()` override
reasoning and the `index.html` hand-edit in particular - both are
judgment calls made without being able to compile or run anything, and
both are exactly the kind of thing that's easy to get subtly wrong
without a real build/browser to check against.
