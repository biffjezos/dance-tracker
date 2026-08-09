RFI-ID: RFI-response-RFC-007-browser-verification-confirmed
Created: 2026-08-09
Created-By: software_architect
Target-Role: software_developer
Related-Specification: SPEC-webgpu-compute-backend / RFC-007
Priority: high
Status: Answered

Subject: RFC-007 confirmed working in a real browser — closing.

Context: RFC-007's own report and the Code Reviewer's evaluation both
flagged AC2 (proving other `App` methods keep working after an
`init_gpu` failure) as unverifiable in either sandbox — source-trace
only, no wasm32 build or browser reachable. Both recommended real-world
confirmation before treating RFC-007 as closed.

Answer: Management rebuilt from `ebaa2b6` (RFC-007's merge commit) and
tested directly. The `requestDevice`-on-null `TypeError` still fires
(expected and unchanged — RFC-007 never claimed to silence that console
error, only the lockup it used to cause, per the RFC's own "Note on the
raw JS throw itself" section). Confirmed: **video loading now works**
after that error fires, where before RFC-007 it did not (this was the
exact "cannot load videos" / "everything is broken" symptom RFC-007 was
filed against). The app.gpu borrow-lockup fix holds under real
conditions, not just under source-level trace.

RFC-007 is closed on my end as of this response. RFI-002 (the standing
question about a headless wasm-bindgen-test harness closing this class
of verification gap for future GPU/wasm-boundary bugs, so this doesn't
require a human's browser each time) is still open separately — answer
that on its own timeline, it isn't blocking RFC-007's closure.

Impact if unanswered: None — this is a closing confirmation, no response
required unless something about the fix's behavior looks different from
what's described here.
