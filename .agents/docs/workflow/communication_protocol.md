# Communication Storage

All inter-agent communication artifacts must be stored in `.agents/communication/`.

Directory structure:

```
.agents/communication/
advice/
adr/
evaluation/
notifications/
rfc/
rfi/
```

Rules:

- Communication artifacts are immutable records.
- Agents must not delete communication artifacts.
- State files reference communication artifacts by ID.

# Session Registry & Delivery

## The registry

`.agents/session_registry/<role_tag>.md` (one file per role) maps a role to
its CCR session and trigger.

Source of truth for "is a session alive" is a live session listing filtered
by `role:<tag>`, not the registry file.

## Session Registration procedure

Run at the start of every session, immediately after reading role
instructions, before reading state.

1. Determine your CCR session ID from the `Claude-Session:` line in the git
   commit template. Do not use a role instructions file's own frontmatter for
   self-identification.
2. Tag yourself `role:<own_role>`.
3. Read `.agents/session_registry/<role_tag>.md`.
4. If another live session already holds this tag: note yourself under
   "Additional live sessions" in the registry file, do not touch the
   trigger, stop.
5. Otherwise, become the pager:
   - No trigger recorded: create one bound to your session. Prompt: check
     `.agents/communication/rfi/` and `.agents/communication/notifications/`
     for items addressed to your role. Daily fallback schedule.
   - Trigger recorded but bound to a different or dead session: delete it,
     create a new one bound to your session, same prompt/schedule.
   - Trigger already bound to your session: no action.
   - Update the registry file: trigger ID, session ID, today's date.
6. Continue with your role's Execution Protocol.

## Delivery

1. Commit the artifact to `.agents/communication/<type>/` on your own
   current branch. Push. Do not wait for a `dev` merge.
2. Read the target role's row in `.agents/session_registry/<role_tag>.md`
   for its trigger ID. If firing it fails, confirm the live trigger via
   `list_triggers` cross-referenced against the role's tag.
3. Call `fire_trigger` on that trigger ID. Payload is exactly: sender role,
   receiver role, sender session ID, sender trigger ID, sender branch name.
   No artifact content, no file path.
4. No trigger ID recorded for the target role: note as a blocker, do not
   assume delivery happened.

Any role may address any role directly for RFI/RFC/Evaluation/Approval/
Implementation Report — not restricted to `governance_and_organization.md`'s
default chain.

## Receiving a Trigger Fire

Act in the same turn the fire arrives.

1. Read the notification for sender role, session ID, trigger ID, branch
   name. `git fetch` that branch.
2. Scan `.agents/communication/**` on that branch for anything addressed
   `Target-Role: <your role>` you have not already answered.
3. Act on it per your role instructions.
4. Respond via the same Delivery procedure: commit your artifact, fire the
   sender's trigger with the same bare notification.
5. Update your working state.

If no fire-back arrives after a reasonable wait, check the target's branch
directly for a committed response.

## Implementation Review Loop

1. Software Architect → Software Developer: RFC, `Target-Role: Software
   Developer`, referencing the specification, describing the implementation
   required.
2. Software Developer implements on its own branch, pushes. On completion →
   Code Reviewer: RFI, `Target-Role: Code Reviewer`, referencing the
   specification and the RFC-ID, asking whether the implementation is ready
   to merge.
3. Code Reviewer evaluates directly on the Developer's branch. Never
   pushes, merges, or modifies `src`/`tests`. Responds:
   - Ready: Approval, `Target-Role: Software Developer`.
   - Not ready: RFC, `Target-Role: Software Developer`, describing the
     required change. Developer fixes and repeats from step 2.
4. On Approval: Software Developer merges to `dev`, records the merge
   commit, sets its own working state to idle, then — as the last action —
   → Software Architect: Implementation Report, `Target-Role: Software
   Architect`, referencing the Approval-ID and the RFC-ID.
5. Software Architect discovers the Implementation Report per "Receiving a
   Trigger Fire" and closes out the RFC.

## Running more than one session per role concurrently

A second session for a role is an additional worker, not a competing pager.
Not yet built, build when needed:

- Add `Claimed-By:` (session ID + timestamp) to an RFI/Notification before
  answering it; re-read to confirm the claim won.
- Switch the role's trigger from `persistent_session_id` to
  `create_new_session_on_fire: true`.

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

Reports a finding the sender has already diagnosed. Used for
infrastructure, environment, sandbox, or tooling conditions that block
work regardless of code or specification correctness. See
`.agents/docs/ENVIRONMENT_DIAGNOSTICS.md` before filing one.

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
