<!-- .agents/docs/session_registry.md -->
---
title: Session Registry
owner_role: management
---
# Session Registry

Maps each role to the CCR session(s) currently embodying it, and the
Routine(s) that page that role's session on new RFIs/Notifications
(see `communication_protocol.md`).

**Source of truth for "is a session alive/current" is the session tag,
checked live** - `list_sessions` filtered by `role:<name>` - not this
file. This file is a fast, human-readable pointer. Verify against tags
before trusting a row that looks stale (more than a day since last
verified, or after any session was created/archived for that role).

## Maintenance responsibility

Not automated. There is no session-lifecycle hook that updates this file
or rebinds a trigger when a role's session is replaced - it requires a
deliberate step by whoever replaces the session. In this project's
current practice that is Management (the human), since Management is the
one calling session creation/archival. If a role session ever spins up
its own replacement, it is equally responsible for the same steps before
considering its own startup complete.

`update_trigger` cannot change which session a Routine is bound to
(`persistent_session_id` is set only at creation) - a session swap always
means delete-and-recreate the trigger, not an in-place update.

### Checklist when replacing a role's session

1. Tag the new session: `role:<role_name>` (`set_session_tags`).
2. `delete_trigger` on the role's existing trigger ID.
3. `create_trigger` a new one bound to the new session ID, reusing the
   same prompt/schedule as the old one (or updating it deliberately, not
   as a side effect of the swap).
4. Update this file's row for that role: new trigger ID, new session ID,
   today's date.

Old/archived sessions keep their tag for history - no cleanup needed
there, only the trigger rebind is required.

## Roles

| Role | Tag | Trigger ID | Session ID (last verified) | Last verified |
|---|---|---|---|---|
| Software Architect | `role:software_architect` | `trig_0131HyDj5tqKLY42Zhu8m84c` | `session_01BHxXDGd8KtzB4DTGX1RxwX` | 2026-08-07 |
| Software Developer | `role:software_developer` | `trig_01RKQE7LDEQYJ7FP6uUtaim3` | `session_019STiQpT9EUN4cetDxE88dW` | 2026-08-07 |
| Code Reviewer (Evaluator) | `role:code_reviewer` | _none yet_ | `session_01SLe29yr62NP2pnQfh8W87k` | 2026-08-07 |
| Management | `role:management` | n/a (human-driven) | `session_014QDjRMamKdHuEKYbSgKY4y` | 2026-08-07 |
| Technical Advisor | `role:technical_advisor` | n/a (no inbound trigger) | `session_01XhxXWKKkhJzcQ4stjJXVho` | 2026-08-07 |

## Running more than one session per role concurrently

Not supported by the trigger design above - `persistent_session_id`
binds a Routine to exactly one session, so two Developer sessions running
at once would need a different pattern than "push to a specific session"
to both be able to pick up an RFI. Recommended shape, **not yet built -
build when actually needed, not speculatively**:

- RFIs/Notifications remain files in `.agents/communication/{rfi,
  notifications}/`, as today.
- Add a `Claimed-By:` field (session ID + timestamp) to those formats. A
  session intending to answer one sets this field first and commits, then
  re-reads the file to confirm its own claim actually won (standard
  optimistic-locking shape for a git-backed queue - whoever's commit
  landed first keeps the claim, the other backs off and looks for a
  different item).
- The wake-up Routine for a multi-session role switches from
  `persistent_session_id` to `create_new_session_on_fire: true`: each
  firing spins a fresh session whose prompt tells it to check for
  unclaimed items tagged for its role and claim+handle one. This works
  regardless of how many long-running sessions exist for that role at the
  time, since it never pushes into a specific one of them.
