<!-- Handoff snapshot, version 1, delivered 2026-08-08. Canonical/owned copy:
     .agents/roles/software_architect/docs/specifications/SPECghostdelay.md
     (Software Architect). Read that file for future updates - this copy is
     the point-in-time delivery instance for this handoff, not a second
     source of truth to maintain independently. -->

SPEC-ID: SPEC-GHOST-DELAY
Created: 2026-08-08
Created-By: software_architect
Target-Role: software_developer
Status: Open

Title: GHOST — fix DELAY to scale per ghost layer (cascading delay), backfill DELAY spec

Purpose:

`engine/src/operations/generators/ghost.rs` never had a written specification
for its `DELAY` parameter — `ANIMATION_IMPLEMENTATION_PLAN.md`'s "GHOST - real
spec" section (the only prior spec for GHOST) predates `DELAY` entirely and
explicitly states "There is no frame-feedback/persistence involved at all."
`DELAY` was added later with no accompanying spec. Management filed RFC-004
(`.agents/communication/rfc/RFC004operationsghostdelay.md`) reporting that
`DELAY` does not behave as intended. This document backfills the missing
specification for `DELAY` and defines the required code change.

Scope:

`Ghost::render_with_cutout` and `Ghost::record_history` in
`engine/src/operations/generators/ghost.rs`, and their associated doc
comments and unit tests. No other file.

Requirements:

## Current (incorrect) behavior

Every ghost layer `n` in `1..=GHOST_COUNT` reads the **same** historical
frame: `render_with_cutout`'s loop calls `self.delayed_cutout(self.delay)`
unconditionally, with no dependency on `n`. Only the spatial offset scales
by `n` (`offset_x = n * distance * spatial_x`, likewise for `y`). This is
deliberate today — both the struct-level doc comment ("DELAY does not
[scale]... every ghost shows the source from the same DELAY frames ago,
same as OPACITY_MULTIPLIER") and the test
`every_ghost_uses_the_same_delay_not_scaled_by_ghost_index` assert it as
correct.

This contradicts RFC-004's worked example: with `GHOST_COUNT = 5`, "only
the first ghost is computed from the previous frame of the source. All
subsequent ghosts compute a delay from the previous GHOST (or ghost
layer)" — a cascading chain, not a shared value.

## Required (correct) behavior

`DELAY` must scale by ghost index `n`, exactly mirroring how `DISTANCE`
already scales by `n`:

```
ghost_n_delay = n * DELAY frames back from the current frame,  for n = 1..=GHOST_COUNT
```

This is the direct mathematical consequence of RFC-004's cascading
description: each ghost layer's own content is a plain time-shifted copy
of the same source-cutout stream (spatial translation does not depend on
time, so it commutes with the delay). Chaining "ghost `n` = ghost `n-1`
delayed by `DELAY` more frames" therefore telescopes to "ghost `n` = source
cutout from `n * DELAY` frames back" — there is no need to actually store
or delay each ghost layer's own rendered buffer; delaying the shared
`history` of source cutouts by `n * DELAY` per layer produces identical
output. This keeps the existing single `history: RefCell<VecDeque<Vec<f32>>>`
design (one shared buffer of past source cutouts) rather than requiring a
separate history stream per ghost layer.

Concretely, in `render_with_cutout`'s loop:

```rust
for n in (1..=self.ghost_count).rev() {
    let delayed = self.delayed_cutout(n as u64 * self.delay);
    // offset_x / offset_y unchanged
    ...
}
```

`record_history`'s capacity must grow to match the deepest ghost's reach.
Its own doc comment already states the correct target ("trim it down to
exactly what the deepest ghost currently needs — `ghost_count * delay`
frames back, plus the current one") — the implementation just never
matched it (`capacity = self.delay as usize + 1`, not scaled by
`ghost_count`). Fix the capacity computation to match that existing
comment: `ghost_count * delay + 1` (guard against `usize` overflow with a
saturating multiply — `GHOST_COUNT`/`DELAY` are both user-editable numbers
with no fixed upper bound in `parameters()`).

`OPACITY_MULTIPLIER` is unaffected by this change — it stays one shared
value applied identically to every ghost, not indexed by `n` (this is a
separate, already-correct, already-tested behavior; do not change it).

## Worked examples (from RFC-004, verified against the fix above)

- `SPATIAL_X = 0, SPATIAL_Y = 0, GHOST_COUNT = 1, DELAY = 0`: ghost 1 reads
  `1 * 0 = 0` frames back (current frame), offset `(0,0)` — renders on top
  of the source, i.e. visually a copy of the source frame. Unchanged by
  this fix (also correct today, since `n=1` is a no-op case for scaling).
- Same, `DELAY = 1`: ghost 1 reads `1 * 1 = 1` frame back — the previous
  frame's mask is visible alongside the current source. Unchanged by this
  fix (also already correct today for `GHOST_COUNT = 1`, since scaling by
  `n=1` doesn't change the value).
- `GHOST_COUNT = 5, DELAY = 1`: ghost 1 reads 1 frame back, ghost 2 reads 2
  frames back, ghost 3 reads 3, ghost 4 reads 4, ghost 5 reads 5 — this is
  where today's behavior is wrong (all five currently read 1 frame back)
  and where this fix changes the output.
- `SPATIAL_X = 5, SPATIAL_Y = 10`: unaffected by this spec — already
  correct today, ghost `n`'s spatial offset is `n * DISTANCE * (5, 10)`.

Acceptance Criteria:

1. Ghost layer `n` (`1..=GHOST_COUNT`) reads its historical cutout from
   `n * DELAY` frames back, not a shared `DELAY` value.
2. `record_history`'s retained capacity covers the deepest ghost's reach
   (`GHOST_COUNT * DELAY` frames back, plus the current frame) so the
   deepest ghost is never starved of real history it should have.
3. `OPACITY_MULTIPLIER` and `SPATIAL_X`/`SPATIAL_Y`/`DISTANCE` behavior is
   unchanged (existing tests for those must still pass unmodified).
4. `DELAY = 0` behavior is unchanged for any `GHOST_COUNT` (`n * 0 = 0`
   for all `n` — every ghost reads the current frame, same as before).
5. The existing test `every_ghost_uses_the_same_delay_not_scaled_by_ghost_index`
   asserted the old (incorrect) behavior — replace it with a test proving
   the new cascading behavior, e.g. `GHOST_COUNT = 2, DISTANCE = 1,
   SPATIAL_X = 1, DELAY = 1`, three ticks (red, green, blue) with
   `show_source = false`: ghost 1 (landing at `x=1`) must show the 1-frame-
   back colour (green), ghost 2 (landing at `x=2`) must show the 2-frame-
   back colour (red) — proving the two ghosts read different history depths.
6. All doc comments that describe DELAY as uniform/shared across ghosts
   (the struct-level doc comment on `Ghost`, lines 29-39, and
   `render_with_cutout`'s own doc comment) are updated to describe the
   corrected per-`n` scaling — do not leave stale prose contradicting the
   new code, per this codebase's own convention of comments explaining the
   non-obvious "why".
7. `cargo build`/`cargo test` succeed for the full existing suite plus the
   new/updated test(s). If the sandbox's network policy blocks dependency
   resolution (see `.agents/communication/notifications/notification_cargo_registry_index_blocked.md`
   and `.agents/docs/ENVIRONMENT_DIAGNOSTICS.md`), record this explicitly
   as unverified in the implementation report rather than silently
   skipping it.

Constraints:

- Only `engine/src/operations/generators/ghost.rs` should require changes
  (implementation + its own doc comments + its own unit tests). No other
  operation, no graph/executor changes — `DELAY`'s history mechanism is
  entirely local to this operation (`is_live()` / cross-tick `history`
  already exist and are unaffected by this fix).
- Do not change `OPACITY_MULTIPLIER`'s semantics (still one shared value,
  not indexed by `n`) — that is separate, correct, already-specified
  behavior (`ANIMATION_IMPLEMENTATION_PLAN.md`'s GHOST section).
- Do not attempt to give each ghost layer its own independent history
  buffer — the single shared `history` of source cutouts, read at `n *
  DELAY` depth per layer, is mathematically equivalent to a cascading
  per-layer delay chain (see Requirements above) and is the simpler
  implementation; a per-layer history stream is not required and would be
  unnecessary complexity.

Dependencies: None — `engine/src/operations/generators/ghost.rs` and its
history mechanism already exist and compile on `dev` (merge commit
`0d2421a` and earlier). This is a self-contained bug fix, not new
scaffolding.

Architecture considerations:

This does not change GHOST's classification in
`SPECwebgpuoperations.md`'s taxonomy (`Stateful/temporal — not recommended
yet` for GPU work) — the fix only changes which history depth each ghost
layer reads, not whether GHOST carries cross-tick state at all.

Testing requirements: See Acceptance Criteria 5 above for the specific new
test scenario. Existing tests not concerning DELAY's per-ghost scaling
(cutout, translate, composite_over, opacity multiplier, zero-delay,
clamping-to-oldest-frame, bbox-consumption-equivalence tests) must
continue to pass without modification — the fix is scoped to the
delay-depth lookup and history capacity only.

Out of scope:

- Any change to `MASK`'s optionality, `SHOW_SOURCE`, `OPACITY_MULTIPLIER`,
  or spatial offset math — all already correct per RFC-004 and unaffected
  by this fix.
- GPU acceleration of GHOST (explicitly deferred, see
  `SPECwebgpuoperations.md`).
- Backfilling a full GHOST specification document covering parameters
  already correctly specified in `ANIMATION_IMPLEMENTATION_PLAN.md` — this
  document only backfills the previously-unspecified `DELAY` behavior.

Open questions: None — RFC-004's worked example plus the codebase's own
pre-existing `record_history` doc comment (which already states the
`ghost_count * delay` capacity target) leave no architectural ambiguity
about the intended behavior.
