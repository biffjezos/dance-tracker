# Evaluation: Phase 1 — Geometric producers report (RESIZE, MOVE)

**Commit:** bfe93c5 on `claude/bbox-phase-1`
**Spec:** `1_bboxawarenessspec.md`, Phase 1 section — no spec files were missing; I still had the full spec from earlier in this session.

## 1. Summary of the change

`RESIZE` and `MOVE` each override `Operation::output_bbox()` to remap their `Source` input's reported box through their own parameters, report-only — pure arithmetic, no pixel reads, `execute()` untouched in both files (confirmed directly: no `execute()` diff hunks in either file).

## 2. Verification against requirements

I checked the diff against the Phase 1 spec, built and ran the test suite myself, and — critically — did not stop at "the listed unit tests pass." For each operation's `output_bbox()`, I independently derived what the *true* content extent is from its `execute()`-path pixel function (`resize_pixels`/`move_pixels`) and checked, by brute-force enumeration across frame sizes/scales/offsets/box positions (thousands of cases per operation, cross-checked against the real Rust code where a violation surfaced), whether the reported box always contains it. This is the one property `BBOX_CONVENTIONS.md` calls the "correctness invariant everything else depends on," and it's exactly the kind of thing that looks right by eyeball but isn't.

- **Diff scope (AC-adjacent, "Diff scope" in the report):** confirmed via `git show --stat` — exactly `resize.rs` and `move_op.rs`, nothing else. ✅
- **`cargo build`/`cargo test`:** ran myself — 238 passed, 0 failed, matching the report exactly. ✅
- **AC1 (`RESIZE` 50% → exactly half, centered):** independently recomputed `remap_x(0)=2, remap_x(8)=6` for an 8×8 frame at 50% — matches the test's `Rect{2,2,6,6}` exactly. ✅
- **RESIZE safety (does the reported box ever undershoot the true content extent?):** brute-forced 5,304 combinations of frame width (4–17), scale (10%–300%), and source-box position/extent, comparing the reported box against the true dest-pixel coverage derived from `resize_pixels`'s own dest→src formula. **Zero violations** — the floor/ceil outward-rounding on the remapped continuous bounds is genuinely safe, including at the edges and for enlargement (scale > 100). This is a correct implementation, not just one that happens to pass its own hand-picked test cases. ✅
- **AC2 (`MOVE` translate + clamp + empty-on-overshoot):** the two tests (`(2,1)` clamped, `(100,0)` → empty) both check out against the code's `intersect(&Rect::full)` logic. ✅ — *for integer offsets.*
- **AC3 (chaining into unmodified `INVERT` is pixel-identical):** verified both graph tests' hand-derived pixels myself against `invert.rs`'s actual math; both are correct, real `Graph`/`PreviewExecutor` tests, not synthetic stand-ins. ✅

## 3. Issues

**Blocking:**

- **`MOVE::output_bbox()` reports a box that can be strictly smaller than the true content extent whenever `OFFSET_X`/`OFFSET_Y` is non-integer — a direct violation of `BBOX_CONVENTIONS.md`'s core safety invariant.** The bug: `output_bbox()` rounds the *offset* to the nearest integer (`self.offset_x.round() as i32`) before translating the box, but `move_pixels` (the actual pixel-sampling code) uses the *exact, unrounded* fractional offset with truncating (`as u32`) sampling. These two roundings disagree whenever the offset has a fractional part, and the box's rounding can land on the wrong side.

  Concrete counter-example, verified against the real code (not just modeled): width=4, `Source` box `[0,1)` (only source pixel 0 is non-transparent), `offset_x = 0.4`. `move_pixels` puts real content at `dest_x=1` (`src_x = 1 - 0.4 = 0.6`, truncates to source pixel `0`, which is inside the box). But `output_bbox()` computes `offset_x.round() = 0` and reports `[0,1)` — a box that does *not* include `dest_x=1`. I added a probe test to the actual crate and confirmed this fails:
  ```
  reported box Rect { x0: 0, y0: 0, x1: 1, y1: 1 } does not cover dest_x=1,
  where move_pixels actually writes real content for offset_x=0.4
  ```
  I also brute-forced this the same way as `RESIZE`: 4,335 combinations of frame width, offset (including fractional and negative), and box position — **970 violations**, all involving a non-integer offset.

  **Why this matters now, not later:** this phase is report-only, so the bug is currently inert — nothing consumes `output_bbox()` yet, so no pixels are actually wrong today. But it directly contradicts the one invariant Phase 3 (and every phase after it) will rely on without re-checking: "must never be smaller than the true extent of non-default content." `OFFSET_X`/`OFFSET_Y` are freely-typed `Number` parameters with no step enforcement at the value level (the operation's own `set_parameter` accepts any `f64`), so a fractional offset is an ordinary, easily-reached user value, not a contrived edge case. If this ships now, Phase 3 will build real compute-skipping on top of a box that's already known to be able to clip real `MOVE` output — and the resulting bug (a sliver of a moved layer silently disappearing) would surface two phases removed from its actual cause, with no test anywhere pointing at it, since every existing `MOVE` bbox test uses only integer offsets (`1.0`, `2.0`/`1.0`, `100.0`).

  **Concrete fix**, mirroring the outward-rounding approach `RESIZE` already uses correctly in this same commit — round the *translated continuous bound*, not the offset:
  ```rust
  let translated = Rect {
      x0: (source_box.x0 as f64 + self.offset_x).floor() as i32,
      y0: (source_box.y0 as f64 + self.offset_y).floor() as i32,
      x1: (source_box.x1 as f64 + self.offset_x).ceil() as i32,
      y1: (source_box.y1 as f64 + self.offset_y).ceil() as i32,
  };
  ```
  I re-ran the same brute-force check against this formula: 0 violations across the same 4,335 cases. Worth also adding a regression test with a fractional offset (e.g. the counter-example above) so this can't silently regress.

No other blocking issues. No major, minor, or nit findings beyond the one above — the rest of the implementation (RESIZE's math, the clamp/empty handling, the diff scope, the AC3 chaining tests) is correct and well-verified by its own test suite.

## 4. What was done well

- `RESIZE`'s outward-rounding (floor lower bound, ceil upper bound, on the *remapped* continuous coordinate) is the right pattern and is genuinely safe under brute-force scrutiny across scale factors from 10% to 300% and arbitrary box positions — this is the harder of the two operations to get right, and it's right.
- The AC3 chaining tests use a real `Graph`/`PreviewExecutor`, with hand-verified expected pixels I independently re-checked and confirmed correct, not just self-consistent.
- The `Rect::empty()` short-circuit (an empty `source_box` doesn't get translated/remapped further) is correct in both operations and avoids a wasted/misleading calculation on already-empty input.
- Diff confinement is exact — only the two files the phase names, no incidental changes.

## 5. Recommendation

**❌ Request changes** — fix `MOVE::output_bbox()`'s rounding (use the outward-floor/ceil pattern `RESIZE` already demonstrates correctly in this same commit) and add a fractional-offset regression test before merging. Everything else in this phase is solid and should not need rework.
