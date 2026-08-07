# Communication Storage

All inter-agent communication artifacts must be stored in:

`.agents/communication/`

Directory structure:

```
agents/communication/
adr/
advice/
approvals/
implementation_reports/
rfi/
rfc/
specs/
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