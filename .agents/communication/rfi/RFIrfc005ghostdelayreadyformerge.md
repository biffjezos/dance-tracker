RFI-ID: RFI-RFC-005-READY
Created: 2026-08-08
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-GHOST-DELAY / RFC-005
Priority: medium
Status: Open

Subject: Is the GHOST DELAY cascading fix (RFC-005) ready to merge?

Context: RFC-005 required `GHOST`'s `DELAY` parameter to scale by ghost
index `n` (`n * DELAY` frames back) instead of every ghost layer reading
the same historical frame. Implemented on branch
`claude/dev-session-plw4fq`, scoped entirely to
`engine/src/operations/generators/ghost.rs` per the spec's constraint.
Full details in
`.agents/communication/implementation_reports/RFC005_ghost_delay_fix_report.md`.

Summary of the change:
1. `render_with_cutout`'s loop now calls `self.delayed_cutout(n as u64 *
   self.delay)` instead of `self.delayed_cutout(self.delay)`.
2. `record_history`'s capacity is now
   `(ghost_count as u64).saturating_mul(delay).saturating_add(1)`,
   matching the pre-existing doc comment's stated target.
3. Both doc comments describing DELAY as shared/uniform updated to
   describe the cascading per-`n` behavior.
4. `every_ghost_uses_the_same_delay_not_scaled_by_ghost_index` (asserted
   the old, incorrect behavior) replaced with
   `each_ghost_delay_scales_by_ghost_index_like_distance_does`, matching
   SPEC-GHOST-DELAY's Acceptance Criterion 5 worked example.

Question: Does this satisfy RFC-005 / SPEC-GHOST-DELAY's acceptance
criteria and constraints? Ready to approve for merge to `dev`?

Reason: `cargo build`/`cargo test` are unverified in my session -
`index.crates.io` is blocked (403, "Host not in allowlist"), matching
`notification_cargo_registry_index_blocked.md`, and `--offline` fails
with no local registry cache. I manually traced the new test's expected
values against the implementation (see the implementation report) but
have not executed the suite. If your session has working `cargo`
access, please run `cargo test --lib generators::ghost` (and ideally the
full suite) to confirm.

Impact if unanswered: RFC-005 stays open past its Implementation
Review Loop step 2; Management's RFC-004 (the original bug report) stays
unresolved.
