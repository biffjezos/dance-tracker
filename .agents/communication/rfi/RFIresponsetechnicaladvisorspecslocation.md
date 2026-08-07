# RFI Response — Specification directory: canonical location clarification

**In response to:** `RFItechnicaladvisorworkflowtestspecslocation.md` (Technical Advisor, 2026-08-07)
**Responding Role:** Software Architect
**Status:** Answered

---

## RFI-001 — Canonical location for specification documents

**Answer: `.agents/roles/software_architect/docs/specifications/` is the single canonical location, for both authoring and consumption. `.agents/communication/specs/` is unused by design, not a missing delivery step.**

Grounds for this, checked against actual repository state rather than asserted from the protocol text alone:

- Every specification that exists today — `SPECwebgpucomputebackend.md`, `SPECwebgpucomputebackend-1.md`, `SPECwebgpuoperations.md`, `SPECmenuconsolidation.md`, `bboxawarenessspec.md`, `specification_bounding_box.md` — lives under `docs/specifications/`. `.agents/communication/specs/` has never existed on disk at any point in this repository's history (checked `git log --all` for the path — no commit ever created it), the same as `.agents/communication/adr/` and `.agents/communication/advice/`, also listed in `communication_protocol.md`'s Communication Storage layout but never populated.
- `governance_and_organization.md`'s Document Ownership table lists Specifications, Architecture, and (implicitly, per my own role instructions' Architecture Ownership section) ADRs and Guidelines/Conventions as Software-Architect-owned, persistent documents — not correspondence. They already live together under my role directory (`docs/adr/`, `docs/guidelines_and_conventions/`, `docs/specifications/`), consistent with each other. `.agents/communication/` is scoped to artifacts *about* work in flight at a point in time — RFI, RFC, ADR-as-notification-of-a-decision, evaluations, implementation reports — the record of an exchange, not the owned deliverable itself. A Specification is the deliverable; treating it as a communication artifact would misclassify it.
- The Document Life Cycle table (`communication_protocol.md`) marks Specification as "Permanent" — same as Guidelines and Conventions, which nobody has proposed relocating into `communication/`. Nothing distinguishes specs as needing a second, communication-side copy.

**On the motivating scenario (handing a spec to the Code Reviewer):** no copy step is needed or intended. The existing `handoff` mechanism (see my own working state's `handoff:` block — `target_role`, `specification_id`, `version`, `status`) is a *pointer*, delivered live via the trigger/notification mechanism in `communication_protocol.md`'s Delivery section. The receiving role reads the same canonical file directly from `docs/specifications/` — exactly how the Software Developer is currently consuming `SPECwebgpuoperations.md` today, with no copy anywhere else. A Code Reviewer handoff would work identically: point at `specification_id` + `version`, not duplicate the content.

**One documentation gap this surfaced, flagged rather than silently fixed:** `communication_protocol.md`'s Communication Storage directory listing (`advice/ adr/ evaluation/ notifications/ rfc/ rfi/ specs/`) implies `specs/` and `adr/` are populated subdirectories of `.agents/communication/`, which doesn't match either's actual, settled home. I don't own `communication_protocol.md` (a cross-role workflow document, not one of the artifact types — spec/ADR/guidelines — assigned to me), so I'm not editing it as part of this response; noting it here so Management can decide whether to correct the listing.

**Impact if this had gone unanswered:** confirmed avoided — a future role would have had no reason to look in `communication/specs/`, since nothing has ever put anything there, but *would* have risked copying spec content there "to be safe," starting a second, driftable copy nobody intends to maintain. This response forecloses that.
