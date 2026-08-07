# Communication Storage

All inter-agent communication artifacts must be stored in:

`.agents/communication/`

Directory structure:

```
agents/communication/
rfi/
rfc/
approvals/
advice/
adr/
specs/
notifications/
```

Rules:

- Communication artifacts are immutable records.
- Agents must not delete communication artifacts.
- Resolved artifacts remain available for traceability.
- State files reference communication artifacts by ID.

# Document Life Cycle

| Document type | Lifetime |
|---|---|
| Specification (Spec) | Permanent |
| Architecture Decision Record (ADR) | Permanent |
| Guidelines and Conventions | Permanent |
| Request for Information (RFI) | Permanent |
| Request for Change (RFC) | Permanent |
| Approval record (Approval) | Permanent |
| Technical Advice | Permanent |
| State files | Current state only; previous versions through git history |
| Plans | Permanent while relevant; archive after completion |
| Parked work | Permanent until completed, cancelled, or removed by Management decision |
| Notifications | Temporary; archive or remove after they are no longer operationally relevant |

# File Formats

---

## Approval

APPROVAL-ID:
Created:
Created-By:
Target-Role:
Related-Specification:
Status:

Commit Reviewed:
Reviewer:
Acceptance Criteria:
Evidence:
Findings:
Decision:

## Architecture Decision Record (ADR)

ADR-ID:
Created:
Created-By:
Target-Role:
Related-Specification:
Status:

Title:
Context:
Decision:
Alternatives considered:
Consequences:
Technical impact:
Related specifications:

---

## Implementation Report

REPORT-ID:
Created:
Created-By:
Target-Role:
Related-Specification:
Status:

Summary:
Implemented:
Files modified:
Architecture notes:
Tests executed:
Test results:
Known limitations:
Specification deviations:
Reviewer notes:

---

## Notification

A Notification reports a finding the sender has already diagnosed - there
is no open question for the receiving role to investigate, only something
to act on or acknowledge. Used primarily for infrastructure, environment,
sandbox, or tooling conditions that block work regardless of how correct
the code or specification is - conditions no specification change can
fix, so they do not belong in an RFI to the Software Architect. See
`.agents/docs/guidelines_and_conventions/ENVIRONMENT_DIAGNOSTICS.md` for
how to distinguish a genuine restriction from a misdiagnosis before filing
one.

NOTIFICATION-ID:
Created:
Created-By:
Target-Role:
Related-Specification:
Status:

Category:
Symptom:
Diagnostic Evidence:
Verdict:
Owner:
Resolution:

---

## Request for Change (RFC)

RFC-ID:
Created:
Created-By:
Target-Role:
Related-Specification:
Priority:
Status:

Severity:
Finding:
Evidence:
Required Change:
Acceptance Condition:

---

## Request for Information (RFI)

RFI-ID:
Created:
Created-By:
Target-Role:
Related-Specification:
Priority:
Status:

Subject:
Context:
Question:
Reason:
Impact if unanswered:

---

## Specification (SPEC)

SPEC-ID:
Created:
Created-By:
Target-Role:
Status:

Title:
Purpose:
Scope:
Requirements:
Acceptance Criteria:
Constraints:
Dependencies:
Architecture considerations:
Testing requirements:
Out of scope:
Open questions:

---

## Technical Advice

ADVICE-ID:
Created:
Created-By:
Target-Role:
Related-Specification:
Status:

Subject:
Context:
Assessment:
Options considered:
Recommendation:
Reasoning:
Risks:
Tradeoffs:
Open questions:
