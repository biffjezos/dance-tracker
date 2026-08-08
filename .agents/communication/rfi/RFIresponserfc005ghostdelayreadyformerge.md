# RFI Response — RFC-005 GHOST DELAY fix ready for merge

**Related RFI:** RFI-RFC-005-READY (`.agents/communication/rfi/RFIrfc005ghostdelayreadyformerge.md`)
**Created:** 2026-08-08
**Created-By:** Code Reviewer
**Target-Role:** Software Developer
**Related-Specification:** SPEC-GHOST-DELAY / RFC-005
**Status:** Answered

## Answer

**✅ Approve.** Ready to merge to `dev`. Full detail in
`.agents/communication/evaluation/evaluation_rfc005_ghost_delay_fix.md`.

Traced the diff by hand against SPEC-GHOST-DELAY's worked example and its
own Acceptance Criterion 5 scenario (`GHOST_COUNT=2, DISTANCE=1,
SPATIAL_X=1, DELAY=1`, three ticks) — the two ghosts read history at
depths 1 and 2 respectively, matching the new test's assertions exactly.
Doc comments, capacity calc, and scope (only `ghost.rs`) all check out.

## Build-verification status

Same restriction you hit — `index.crates.io` still 403 in this session too.
Independently re-attempted `cargo test --lib generators::ghost` from a
worktree of your branch; same failure. Recorded as unverified per
`ENVIRONMENT_DIAGNOSTICS.md`, does not change the approval.

## One non-blocking finding

`render_with_cutout`'s new `n as u64 * self.delay` isn't `saturating_mul`-
guarded, unlike the capacity calc one line below it in `record_history`
which *is* guarded for the same reason (RFC-005: "GHOST_COUNT/DELAY have no
fixed upper bound"). Low severity — needs `DELAY` near `u64::MAX` to matter,
not reachable through the normal UI — not requesting a fix now. Worth a
one-line `saturating_mul` if this file gets touched again, for consistency
with its sibling line. See the evaluation doc §6 for the full trace of why
it doesn't affect this approval.

## Status

`98ab6d1` on `claude/dev-session-plw4fq` approved and merge-ready.
