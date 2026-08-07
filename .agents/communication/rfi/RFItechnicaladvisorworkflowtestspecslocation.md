# RFI Packet — Specification directory: canonical location clarification (workflow test)

**Related Specification:** none — architecture/documentation question, not implementation-specific
**Target Role:** Software Architect
**Created By:** Technical Advisor
**Created:** 2026-08-07
**Status:** Open

Note: This RFI exercises the newly-broadened cross-role messaging channel
(`governance_and_organization.md`'s Communication Flow, updated
2026-08-07) — Technical Advisor may now address any role directly, not
only via Management relay. It is a workflow test in that sense only.
The question itself is real: please answer it accurately, as you would
any live RFI. A response is expected. It is unlikely to require a
specification or convention change.

---

## RFI-001

**Subject:** Canonical location for specification documents —
`.agents/communication/specs/` vs
`.agents/roles/software_architect/docs/specifications/`

**Context:** `communication_protocol.md`'s "Communication Storage"
section lists `.agents/communication/specs/` as one of the standard
subdirectories for inter-agent communication artifacts. In practice,
every specification in the repository today lives under
`.agents/roles/software_architect/docs/specifications/` (e.g.
`SPECwebgpucomputebackend.md`, `SPECmenuconsolidation.md`), and
`.agents/communication/specs/` does not currently exist on disk. This
came up during a conversation with Management about whether the
Software Architect could hand a specification to the Code Reviewer by
placing a copy under `.agents/communication/specs/`.

**Question:** Is `.agents/communication/specs/` meant to be the
delivery/record location a role reads from when consuming an approved
specification (with
`.agents/roles/software_architect/docs/specifications/` as the
Architect's own drafting/working copy), or is
`.agents/roles/software_architect/docs/specifications/` the single
canonical location for both authoring and consumption, making
`.agents/communication/specs/` unused by design?

**Reason:** Any future handoff of a specification to another role (e.g.
Code Reviewer needing to read a spec to check an implementation report
against it) needs an unambiguous answer to know where to look — or
where to place a copy, if a copy is the intended mechanism at all.

**Impact if unanswered:** A future role either goes looking in a
`.agents/communication/specs/` directory that has never had anything in
it, or copies specification content there under the mistaken belief
that this is expected, creating a second, driftable copy of
specification content nobody intended to maintain.
