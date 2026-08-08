REPORT-ID: REPORT-RFC-005
Created: 2026-08-08
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-GHOST-DELAY / RFC-005
Status: Awaiting Code Review (supporting detail for RFI-RFC-005-READY)

Summary: Fixed `GHOST`'s `DELAY` parameter to cascade per ghost layer
(`n * DELAY` frames back) instead of every ghost reading the same
historical frame, per RFC-005 / SPEC-GHOST-DELAY.

Implemented:

1. `render_with_cutout`'s loop: `self.delayed_cutout(self.delay)` ->
   `self.delayed_cutout(n as u64 * self.delay)`.
2. `record_history`'s capacity: `self.delay as usize + 1` ->
   `(self.ghost_count as u64).saturating_mul(self.delay).saturating_add(1) as usize`
   (saturating arithmetic, since `GHOST_COUNT`/`DELAY` are both
   user-editable with no fixed upper bound).
3. Updated the two doc comments that described DELAY as shared/uniform
   across ghosts (the `Ghost` struct's top doc comment and
   `render_with_cutout`'s own doc comment) to describe the corrected
   per-`n` cascading scaling instead.
4. Replaced `every_ghost_uses_the_same_delay_not_scaled_by_ghost_index`
   (asserted the old, incorrect behavior) with
   `each_ghost_delay_scales_by_ghost_index_like_distance_does`, per
   SPEC-GHOST-DELAY's Acceptance Criterion 5: `GHOST_COUNT = 2, DISTANCE =
   1, SPATIAL_X = 1, DELAY = 1`, three ticks (red, green, blue) with
   `show_source = false` - ghost 1 (x=1) must show green (1 frame back),
   ghost 2 (x=2) must show red (2 frames back).
5. `OPACITY_MULTIPLIER` untouched, as required.

Files modified:

- `engine/src/operations/generators/ghost.rs`

Architecture notes: No new state, no new mechanism - reuses the existing
single shared `history: RefCell<VecDeque<Vec<f32>>>` buffer, per
SPEC-GHOST-DELAY's explicit instruction not to give each ghost layer its
own history stream. Scoped entirely to this one file, as required.

Tests executed: `cargo test --lib generators::ghost` (and `--offline`
fallback).

Test results: **Unverified** - blocked by sandbox network policy.
`index.crates.io` 403s ("Host not in allowlist"), matching the already-
filed `notification_cargo_registry_index_blocked.md`; `--offline` also
fails (`no matching package named 'inventory' found`, no local registry
cache in this session). Verified `ENVIRONMENT_DIAGNOSTICS.md` before
concluding this is infrastructure, not a code problem. In lieu of running
the suite, manually traced the new test's expected values against the
implementation:
- History after 3 renders (red, green, blue), `ghost_count=2, delay=1`:
  capacity = `2*1+1 = 3`, so all three frames retained, `history =
  [red, green, blue]`, `last = 2`.
  - Ghost 1: `frames_back = 1*1 = 1` -> `index = 2-1 = 1` -> green. Matches
    assertion.
  - Ghost 2: `frames_back = 2*1 = 2` -> `index = 2-2 = 0` -> red. Matches
    assertion.
- Re-checked capacity formula against every other existing DELAY test
  (`zero_delay_uses_the_current_frame...`, `delay_shows_an_older_frame...`,
  `delay_clamps_to_the_oldest_available_frame...`) - all use
  `ghost_count = 1`, where `ghost_count * delay + 1 == delay + 1` (the old
  formula), so none of their expected capacities change.

Known limitations: `cargo build`/`cargo test` unverified per above -
recorded here rather than skipped, per RFC-005's own instruction and
`ENVIRONMENT_DIAGNOSTICS.md`. No functional limitation in the change
itself.

Specification deviations: None.

Reviewer notes: Diff is scoped to
`engine/src/operations/generators/ghost.rs` only, as SPEC-GHOST-DELAY
requires. Please compile/run the suite in your own session if network
access allows, to confirm the manual trace above.
