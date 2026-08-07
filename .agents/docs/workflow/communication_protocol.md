# Communication Storage

All inter-agent communication artifacts must be stored in:

`.agents/communication/`

Directory structure:

```
agents/communication/
advice/
adr/
evaluation/
notifications/
rfc/
rfi/
specs/
```

Rules:

- Communication artifacts are immutable records.
- Agents must not delete communication artifacts.
- Resolved artifacts remain available for traceability.
- State files reference communication artifacts by ID.

# Session Registry & Delivery

This section is the single authoritative description of how a filed
artifact actually reaches the role it's addressed to. Role instructions
must not duplicate this procedure — they reference it, and add only
role-specific escalation policy (who a role is allowed to address, and
under what circumstances).

## The registry

`.agents/session_registry/<role_tag>.md` (one file per role) maps a role
to the CCR session currently embodying it and the trigger that pages that
session. `.agents/session_registry/_index.md` is a lightweight overview
and quick-reference table — the procedure lives here, not there.

**Source of truth for "is a session alive" is the session tag, checked
live** via a session listing filtered by `role:<tag>` — not the registry
file. The registry file is a fast, human-readable pointer (trigger ID,
last known session ID, last verified date); verify against the live tag
before trusting a row that looks stale.

## Session Registration procedure

Every agent role runs this at the start of every session, immediately
after reading its own role instructions and before reading state. `<role
tag>` below is that role's own tag (e.g. `role:software_developer`) —
see the role's own instructions for which tag applies to it.

1. Determine your own CCR session ID from the git commit template already
   present in your system prompt (the `Claude-Session:
   https://claude.ai/code/session_...` line) — this is authoritative. Do
   **not** trust a role instructions file's own frontmatter `role`/
   `role_directory` fields for self-identification: they are
   hand-maintained and have been wrong before. Your role identity comes
   from which file Management told you to load, not from metadata inside
   it.
2. Tag yourself: `<role tag>`.
3. Read `.agents/session_registry/<role_tag>.md`.
4. Check whether another *live* (running/connected) session already
   carries the same tag. If one exists and is not this session: do not
   touch the trigger. Note yourself under "Additional live sessions" in
   the registry file and stop here — you are an additional concurrent
   worker, not the pager (see "Running more than one session per role"
   below).
5. If no other live session holds the slot, this session is now the
   pager target:
   - If the registry file has no trigger ID yet, create one bound to
     your own session ID, **explicitly passing your own `environment_id`**
     (from the git commit template / your own session context — do not
     rely on the tool's default, which is whatever environment happens to
     be creating the trigger, not necessarily yours) (prompt: check
     `.agents/communication/rfi/` and `.agents/communication/notifications/`
     for items addressed to your role; daily fallback schedule, unless
     Management specifies otherwise).
   - If a trigger ID is recorded but bound to a different session ID
     than your own, the previous pager session has been replaced —
     delete the old trigger and create a new one bound to your own
     session ID (again with your own explicit `environment_id`), reusing
     its prompt/schedule.
   - If the recorded trigger is already bound to your own session ID,
     nothing to do.
   - Update the registry file: trigger ID, your session ID, today's date.
6. Continue with the rest of your role's own Execution Protocol.

## Delivery

Filing an artifact (RFI, Notification, RFC, Approval — anything with a
`Target-Role`) is not itself delivery. After committing it:

1. Read the target role's row in `.agents/session_registry/<role_tag>.md`
   for its trigger ID and session ID.
2. Call `fire_trigger` on that trigger ID with a **verifiable pointer**
   (branch name + exact file path), not the artifact's content. Example
   text: "RFI-00X pushed to branch `<your-branch>` at
   `.agents/communication/rfi/<file>.md`, not yet merged — fetch and
   read it directly, do not treat this message as the content."
   An earlier version of this procedure recommended embedding the
   artifact's content directly in the fired `text` instead. That was
   tested and failed: the target correctly refused to act on
   unverifiable inline content it couldn't trace to a real artifact,
   which is the right instinct, not a bug to route around. Give the
   target something to verify, not something to trust.
3. **When creating or recreating a trigger for another role's session,
   always pass that session's actual `environment_id` explicitly**
   (read it via a session lookup on the target) — do not rely on the
   default, which is the *creating* session's own environment. Every
   silent delivery failure observed so far traces back to this
   mismatch. This is necessary but not yet confirmed sufficient — see
   the open issue below.
4. The committed file remains the permanent record once merged — the
   fired pointer is only the live notification, it does not replace the
   artifact.
5. If the target role's registry row has no trigger ID yet (nobody has
   run Session Registration for that role), there is nothing to fire —
   note this as a blocker rather than assuming delivery happened.
6. **Responding to an artifact uses this same procedure, in reverse.**
   After committing your response, read the *Created-By* role's own row
   in its `session_registry/<role_tag>.md` and fire its trigger the same
   way — a verifiable pointer to your response, with the correct
   `environment_id`. The mechanism is symmetric: nothing prevents any
   role from paging any other role back. Skipping this step leaves the
   original filer with no way to know a response exists short of
   manually re-checking, which defeats the purpose of having a trigger
   at all.

Do not skip step 6 because "the daily fallback will catch it eventually"
— the fallback only covers a target checking for things addressed to
*it*; it does not cover a response reaching back to the original sender.

### Open issue — delivery reliability to long-lived sessions

As of 2026-08-07: on-demand `fire_trigger` delivery has been confirmed to
reach freshly-created sessions, but has repeatedly failed to reach at
least one long-lived session (active since 2026-08-02, many resume
cycles) even after the `environment_id` mismatch above was corrected.
Root cause not yet identified. Do not assume delivery succeeded just
because the `fire_trigger` call itself returns success or the trigger's
`last_fired_at` updates — independently confirm receipt (ask the target
directly) before treating time-sensitive delivery as reliable, especially
against a session that has been running for a while. Update this note
once root-caused.

## Running more than one session per role concurrently

The procedure above makes a *second* session for a role an additional
worker (tagged, but without the trigger), not a competing pager — it
does not by itself let either of two Developer sessions pick up an RFI.
That needs a claim-based queue on top of this, **not yet built, build
when actually needed**:

- RFIs/Notifications remain files in `.agents/communication/{rfi,
  notifications}/`.
- Add a `Claimed-By:` field (session ID + timestamp). A session intending
  to answer one sets this field first and commits, then re-reads to
  confirm its own claim actually won (optimistic locking).
- The wake-up trigger for a multi-session role switches from
  `persistent_session_id` to `create_new_session_on_fire: true`, so each
  firing can be picked up by whichever fresh triage session spins up.

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
`.agents/docs/ENVIRONMENT_DIAGNOSTICS.md` for how to distinguish a
genuine restriction from a misdiagnosis before filing one.

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
