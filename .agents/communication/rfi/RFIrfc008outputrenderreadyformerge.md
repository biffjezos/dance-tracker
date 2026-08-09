RFI-ID: RFI-RFC-008-READY
Created: 2026-08-09
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-OUTPUT-RENDER-V1 / RFC-008
Priority: medium
Status: Open

Subject: Is the OUTPUT menu + RENDER node v1 implementation (RFC-008) ready to merge?

Context: RFC-008 (new feature, not a bug fix): a new OUTPUT menu
category, a `RENDER` operation with an empty EDIT screen plus one
EXECUTE button, reactivating the existing (previously unreachable)
`Recorder`/`toggleRecord` mechanism, and removal of the now-redundant
`REC` status-bar field. Implemented on branch `claude/dev-session-plw4fq`.
Full detail in
`.agents/communication/implementation_reports/RFC008_output_render_v1_report.md`.

Three things worth your specific attention, all flagged in detail in the
report:

1. **`supports_edit()` override.** The spec never mentions this method,
   but its default (`false` for zero inputs/params) would make RENDER's
   EDIT button - and therefore EXECUTE - permanently unreachable,
   contradicting the RFC's own AC3/AC4. Overrode it to `true` on
   `Render`. Small, uses an existing trait extension point, but not
   something the spec asked for explicitly - please confirm this
   reasoning holds.
2. **Statusbar index-shift regression, not mentioned in the spec.**
   Removing the `REC` span shifts every later status-bar child's index
   by one; `output.js`'s `applyOutputSize()` wrote to `bar.children[4]`
   by raw index, which would have silently broken the resolution readout
   if left unfixed. Updated it to `children[3]`. Please double-check this
   arithmetic against the final `statusbar.hbs`/`index.html` child order.
3. **`ui/index.html` hand-edited, not build-regenerated** - `npm install`
   fails (`registry.npmjs.org` 403), so I couldn't run the actual
   `npm run build` step. Applied the identical span removal by hand.
   Please diff this file specifically rather than assuming it's
   verified build output.

Question: Does this satisfy SPEC-OUTPUT-RENDER-V1's acceptance criteria
and RFC-008's required changes? Ready to approve for merge to `dev`?

Reason: `cargo build`/`cargo test` unverified - same `index.crates.io`
block as RFC-005/006/007's precedent. No live browser click-through
possible either - the app needs the compiled WASM module, and rebuilding
it hits the same blocked toolchain path (`NOTIFICATION-002`). New Rust
tests (descriptor/metadata/parameters/supports_edit/execute-stub/graph-
validity) and a manual trace through the full menu -> EDIT -> EXECUTE
click path are the available confidence for this round - see the report
for the full trace.

Impact if unanswered: RFC-008 stays open past its Implementation Review
Loop step 2.
