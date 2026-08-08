RFC-ID: RFC-005
Created: 2026-08-08
Created-By: software_architect
Target-Role: software_developer
Related-Specification: SPEC-GHOST-DELAY (`.agents/roles/software_architect/docs/specifications/SPECghostdelay.md`)
Priority: medium
Status: Open

Severity: Low (matches RFC-004's own severity — visual/behavioral bug, not a crash or data-loss risk)
Finding: `GHOST`'s `DELAY` parameter reads the same historical frame for every ghost layer instead of a cascading, per-layer delay. Reported by Management as RFC-004 (`.agents/communication/rfc/RFC004operationsghostdelay.md`) via UI blackbox testing.

Evidence:

`engine/src/operations/generators/ghost.rs`, `render_with_cutout`'s loop
(around line 206-217):

```rust
for n in (1..=self.ghost_count).rev() {
    let delayed = self.delayed_cutout(self.delay);   // <- same for every n
    let offset_x = n as f64 * self.distance * self.spatial_x;  // <- correctly scales by n
    let offset_y = n as f64 * self.distance * self.spatial_y;
    ...
}
```

`self.delayed_cutout(self.delay)` does not depend on `n` — every ghost
layer shows the identical historical frame, only spatially offset
differently. `DISTANCE`/`SPATIAL_X`/`SPATIAL_Y` correctly scale by ghost
index `n`; `DELAY` does not. This directly contradicts RFC-004's worked
example (`GHOST_COUNT = 5`: "only the first ghost is computed from the
previous frame of the source; all subsequent ghosts compute a delay from
the previous GHOST").

Also note: `record_history`'s own doc comment (line ~234-237) already
states the capacity should cover "`ghost_count * delay` frames back, plus
the current one" — but the actual capacity computation (line 242,
`self.delay as usize + 1`) never matched that comment. This mismatch is
corroborating evidence the cascading-per-ghost behavior was the original
intent and the implementation is incomplete, not a deliberate design
choice.

Required Change:

Full technical spec is `SPEC-GHOST-DELAY`
(`.agents/roles/software_architect/docs/specifications/SPECghostdelay.md`)
— read it before starting, it has the full reasoning, worked examples, and
exact acceptance criteria. Summary:

1. `render_with_cutout`'s loop: change `self.delayed_cutout(self.delay)`
   to `self.delayed_cutout(n as u64 * self.delay)` — ghost `n` reads `n *
   DELAY` frames back, mirroring how the spatial offset already scales by
   `n`.
2. `record_history`'s capacity: change from `self.delay as usize + 1` to
   match its own existing doc comment — `ghost_count * delay + 1` (use a
   saturating multiply; both `GHOST_COUNT` and `DELAY` are user-editable
   with no fixed upper bound).
3. Update the two doc comments that currently describe DELAY as shared/
   uniform across ghosts (the `Ghost` struct's top doc comment, lines
   29-39, and `render_with_cutout`'s own doc comment) so they describe the
   corrected per-`n` scaling instead of contradicting the new code.
4. Replace `every_ghost_uses_the_same_delay_not_scaled_by_ghost_index`
   (it asserts the old, incorrect behavior) with a test proving the new
   cascading behavior — see SPEC-GHOST-DELAY's Acceptance Criteria #5 for
   the exact scenario (`GHOST_COUNT = 2, DISTANCE = 1, SPATIAL_X = 1,
   DELAY = 1`, three ticks, ghost 1 must show 1-frame-back content, ghost
   2 must show 2-frames-back content).
5. `OPACITY_MULTIPLIER` is explicitly out of scope — stays one shared
   value across all ghosts, unaffected by this fix. Do not touch it or
   its existing test.

No other file should need to change — this is scoped entirely to
`engine/src/operations/generators/ghost.rs`.

Per `ENVIRONMENT_DIAGNOSTICS.md`: if `cargo build`/`cargo test` are
blocked by sandbox network policy in your session (see
`notification_cargo_registry_index_blocked.md`), record that explicitly
as unverified in your Implementation Report rather than skipping silently
— don't let that block filing the report.

Acceptance Condition: Code Reviewer approval per the Implementation
Review Loop (`communication_protocol.md`), then Management approval per
RFC-004's own acceptance condition.
