<!-- .agents/session_registry/session_registry__index.md -->
---
title: Session Registry — Index
owner_role: management
---
# Session Registry

Split one file per role (`.agents/docs/session_registry/<role_tag>.md`)
so the existing path-based permission model enforces "a role may only
write its own file" structurally, instead of relying on a documented but
unenforced "only touch your own row" rule in one shared file.

**Source of truth for "is a session alive/current" is the session tag,
checked live** via session listing filtered by `role:<tag>` - not these
files. These files are fast, human-readable pointers (trigger ID, last
known session ID, last verified date). Verify against the live tag query
before trusting a row that looks stale.

## Roles

| Role | File | Tag |
|---|---|---|
| Software Architect | `software_architect.md` | `role:software_architect` |
| Software Developer | `software_developer.md` | `role:software_developer` |
| Code Reviewer | `code_reviewer.md` | `role:code_reviewer` |
| Management | `management.md` | `role:management` |
| Technical Advisor | `technical_advisor.md` | `role:technical_advisor` |

## Maintenance: claim-based self-registration, not manual upkeep

Each agent role's own instructions now include a `## Session Registration`
procedure, run at the start of every session, before resuming assigned
work. Summary of the logic (full procedure lives in each role's own
instructions file, not duplicated here to avoid drift):

1. Determine own session ID (from the git commit template already
   present in the system prompt - do not trust a role file's own
   frontmatter for self-identification, see note below).
2. Tag self `role:<own_role>`.
3. Check whether another *live* session already carries the same tag.
   - If yes: this is an additional concurrent worker, not the pager -
     note self under "Additional live sessions" in the role's registry
     file, do not touch the trigger, stop.
   - If no: this session becomes the pager target - create or rebind the
     role's trigger to this session's ID, update the registry file
     (trigger ID, session ID, date).

This is a claim, not a blind register - it exists specifically so two
sessions of the same role (see "Running more than one session per role"
below) don't fight over the trigger binding, and so a stale/replaced
session doesn't leave a dead trigger silently pointing nowhere.

Management is still the actor who decides *when* to spin up a
replacement session for a role - this automation only makes what happens
next (tagging, trigger rebind, registry update) self-executing instead
of a manual checklist item.

### Why not trust a role instructions file's own frontmatter for identity

Found while building this: `instructions_software_architect.md`'s
frontmatter `role`/`role_directory`/`role_file` fields incorrectly
identified it as the Code Reviewer (copy-paste error - permissions and
outputs were correctly Architect-flavored, only the identity fields were
wrong). A self-registration procedure that trusted a file's own claimed
identity rather than which file it was actually told to load would have
silently mistagged that session. Fixed 2026-08-07; keep deriving role
identity from assignment, not from self-reported metadata, regardless.

## Running more than one session per role concurrently

Still not automatically load-balanced - the claim procedure above means
a *second* session for a role becomes an additional worker (tagged, but
without the trigger), not a competing pager. For either of two Developer
sessions to actually be able to pick up an RFI, the claim-queue pattern
below is still needed on top of this - **not yet built, build when
actually needed**:

- RFIs/Notifications remain files in `.agents/communication/{rfi,
  notifications}/`.
- Add a `Claimed-By:` field (session ID + timestamp). A session intending
  to answer one sets this field first and commits, then re-reads to
  confirm its own claim actually won (optimistic locking).
- The wake-up Routine for a multi-session role switches from
  `persistent_session_id` to `create_new_session_on_fire: true` so each
  firing can be picked up by whichever fresh triage session spins up,
  rather than needing to know which long-running session should get it.
