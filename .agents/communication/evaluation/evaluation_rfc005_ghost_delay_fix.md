# Evaluation: RFC-005 — GHOST DELAY cascading fix (RFI-RFC-005-READY)

**Branch:** `claude/dev-session-plw4fq` (not yet merged), commit `98ab6d1`
**Spec:** `SPEC-GHOST-DELAY` (backfilled by Software Architect, delivered copy at
`.agents/communication/specs/SPECghostdelay.md`)
**RFC:** `RFC005ghostdelayfix.md`
**Report:** `.agents/communication/implementation_reports/RFC005_ghost_delay_fix_report.md`
**File touched:** `engine/src/operations/generators/ghost.rs` only — confirmed via
`git diff dev..claude/dev-session-plw4fq -- engine/`, no other file under `engine/` changed.

## 0. Build-verification status — independently reconfirmed, still blocked

Re-ran the developer's own checks in this session, plus attempted the actual
test run:

```
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://index.crates.io/config.json   -> 403
cd engine && cargo test --lib generators::ghost
  -> error: failed to get `bytemuck` as a dependency ...
     download of config.json failed ... 403, "Host not in allowlist"
```

Same restriction as `notification_cargo_registry_index_blocked.md`. **AC7
(cargo build/test) is UNVERIFIED, not confirmed passing** — per
`ENVIRONMENT_DIAGNOSTICS.md`, this is recorded rather than blocking the
review. AC1–AC6 below are verified by direct manual trace against the real
diff, not by compiling.

## 1. Scope compliance

Diff is exactly `ghost.rs`: two doc comments, the `delayed_cutout` call site,
the `record_history` capacity expression, and one test replaced (old
name/assertions removed, new name/assertions added). No other operation, no
graph/executor file touched. Matches RFC-005's constraint exactly. ✅

## 2. Core logic — traced against AC1/AC2/AC4

`render_with_cutout`: `self.delayed_cutout(self.delay)` → `self.delayed_cutout(n as u64 * self.delay)`.
`delayed_cutout(frames_back)` computes `index = last.saturating_sub(frames_back)`
against the shared `history` buffer — exactly the "single shared history,
read at `n * DELAY` depth per layer" design SPEC-GHOST-DELAY calls
mathematically equivalent to a real per-layer cascade. ✅ (AC1)

`record_history`'s capacity: `self.delay as usize + 1` →
`(ghost_count as u64).saturating_mul(delay).saturating_add(1) as usize`,
matching the pre-existing doc comment's stated target
(`ghost_count * delay + 1`) that RFC-005's evidence section flagged as
previously mismatched. Saturating arithmetic used, per the spec's explicit
requirement ("GHOST_COUNT/DELAY are user-editable with no fixed upper
bound"). ✅ (AC2)

`DELAY = 0`: `n * 0 = 0` for every `n` regardless of scaling — unaffected by
this change algebraically, no special-casing needed or added. ✅ (AC4)

## 3. New test — hand-traced against the real history buffer, not just read

Scenario: `ghost_count: 2, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0,
opacity_multiplier: 1.0, delay: 1`. Capacity = `2*1+1 = 3`.

- Frame 0 (red): history = `[red]`.
- Frame 1 (green): history = `[red, green]` (len 2 ≤ capacity 3, no trim).
- Frame 2 (blue): history = `[red, green, blue]` (len 3 = capacity, no trim
  yet — this is the tick the assertions read).
  - Ghost 1 (`n=1`, offset x=1): `frames_back = 1*1 = 1`,
    `index = 2 - 1 = 1` → **green**. Test asserts `out[5]` (pixel 1's G
    channel) `≈ 1.0`. Matches. ✅
  - Ghost 2 (`n=2`, offset x=2): `frames_back = 2*1 = 2`,
    `index = 2 - 2 = 0` → **red**. Test asserts `out[8]` (pixel 2's R
    channel) `≈ 1.0`. Matches. ✅

This is the exact scenario SPEC-GHOST-DELAY's Acceptance Criterion 5
prescribes, and the two ghosts read provably different history depths — not
just two assertions that happen to both pass by coincidence (traced the
buffer contents by hand at each tick, not just the final read). ✅ (AC5)

## 4. Doc comments — AC6

Both flagged locations (`Ghost` struct doc comment, `render_with_cutout`'s
own doc comment) now describe the cascading per-`n` behavior and explicitly
contrast it with `OPACITY_MULTIPLIER` remaining shared — no stale prose
contradicting the new code. ✅

## 5. Out-of-scope behaviors — AC3

`OPACITY_MULTIPLIER`, `SPATIAL_X`/`SPATIAL_Y`/`DISTANCE` code paths are
untouched by the diff (confirmed via the diff itself — those lines don't
appear). Their existing tests are not present in the diff, so they run
unmodified. Cannot execute them (§0), but no code path they exercise
changed. ✅ (as verified as possible without a build)

## 6. Finding — non-blocking, not in scope of SPEC-GHOST-DELAY's ACs

`record_history`'s capacity calc uses `saturating_mul`/`saturating_add`
specifically because "`GHOST_COUNT`/`DELAY` are both user-editable numbers
with no fixed upper bound" (RFC-005's own text, confirmed against
`parameters()`: both `GHOST_COUNT` and `DELAY` have `max: None`). The new
`render_with_cutout` line performing the *identical-shaped* multiplication —
`n as u64 * self.delay` — uses plain `*`, not `saturating_mul`. Under the
same "no fixed upper bound" input space RFC-005 itself invokes to justify
the capacity guard, an extreme `DELAY` could overflow this multiplication:
a debug build panics, a release build wraps to an arbitrary `u64` that
`delayed_cutout`'s `saturating_sub` would then silently (mis)clamp against
history length — not a crash, but not `n * DELAY` either.

Severity: low. Reaching this requires a `DELAY` near `u64::MAX`, not a
realistic value through the UI's number input, and it's not among
SPEC-GHOST-DELAY's stated acceptance criteria (which only require the
guard on the capacity calc). Not blocking approval. Recommend a one-line
`saturating_mul` fix as a fast-follow if `ghost.rs` is touched again, for
consistency with the guard one line above it.

## Decision

**Approve.** AC1–AC6 verified by direct trace; AC7 unverified due to
environment (independently reconfirmed, not a codebase issue). No critical
defects. §6 is a minor, non-blocking observation for future hygiene, not a
gate on this merge.
