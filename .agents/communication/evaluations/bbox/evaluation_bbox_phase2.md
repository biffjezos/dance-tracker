# Evaluation: Phase 2 — BLUR grows its reported bbox (report-only)

**Commit:** 2bab31a on `claude/bbox-phase-2`
**Spec:** `1_bboxawarenessspec.md`, Phase 2 section (still held from earlier in this session)

## 1. Summary of the change

`BLUR::output_bbox()` grows its `Source` input's reported box by `radius_px` on every side via `Rect::grow`, clamped to the frame — report-only, `execute()`/`blur_pixels` untouched.

## 2. Verification against requirements

Given Phase 1's `MOVE` bug was exactly this kind of "looks right, isn't" safety violation, I applied the same brute-force-against-real-code rigor here rather than trusting the analytical justification in the report.

- **Diff scope:** confirmed via `git show --stat` — exactly `blur.rs`, nothing else. ✅
- **`execute()`/`blur_pixels` unchanged:** confirmed directly — no `fn execute` diff hunks anywhere in the commit. ✅
- **Build/test:** ran myself — 244 passed, 0 failed, matching the report exactly. ✅
- **The Phase-1-lesson question — is `radius_px` genuinely always a whole integer, so there's no `MOVE`-style rounding mismatch between the parameter and the box math?** Checked directly: `radius_px: u32` is the stored field, and `set_parameter("RADIUS", ...)` does `self.radius_px = v.round() as u32` — so *any* value the user types (fractional or not) is rounded to an integer at set-time, before it's ever stored. `output_bbox()`'s `self.radius_px as i32` and `blur_pixels`'s `self.radius_px as usize` both read that same already-integer field — there is no place for the two to disagree, unlike `MOVE` where the raw `f64` offset was stored unrounded and only the bbox math re-rounded it independently. The report's claim that this bug class doesn't apply here is verified, not just asserted. ✅
- **Is "grow by radius" actually the correct safe bound for this specific two-pass separable kernel (not just a single-pass 2D blur)?** `blur_pixels` runs a horizontal pass into `tmp`, then a vertical pass on `tmp`. I worked through the substitution: `out(x,y)` depends on `tmp(x, y±r)`, and each `tmp(x, y')` depends on `pixels(x±r, y')` — so `out(x,y)`'s full dependency set is exactly `pixels[x-r..x+r, y-r..y+r]`, the same square window a direct single-pass 2D box blur would use. Growing the box by `r` on every side is therefore the exact right bound, not an approximation. Edge clamping (`saturating_sub`, `.min(w-1)`) only *shrinks* the averaging window near frame edges — it never pulls in pixels beyond radius `r`, so it can't undermine the safety argument.
  - I then verified this empirically against the actual `blur_pixels` function directly (not my derivation) in a throwaway probe test: built pixel buffers with real (non-zero) content confined to various source boxes (full-frame, interior, single corner pixels, a vertical strip) across 5 frame sizes and radii 0–5, ran the real `blur_pixels`, and checked every resulting non-zero output pixel against `output_bbox()`'s reported box. **125 trials, 0 violations.** ✅
- **AC1 (sub-frame box grown by radius, clamped):** re-derived both tests' expected values by hand — `[3,3,7,7)` grown by 2 → `[1,1,9,9)` (no clamp needed, matches); `[0,0,3,3)` grown by 5 → unclamped `[-5,-5,8,8)` → clamped to `[0,0,8,8)` (matches). ✅
- **AC2 (full-frame source stays full-frame after growth):** trivially correct and confirmed by test. ✅

## 3. Issues

None found — no blocking, major, minor, or nit issues. This phase is materially simpler than Phase 1 (integer-only arithmetic, no continuous-to-discrete rounding step), and the implementor's own report explicitly checked for the Phase-1-class bug before claiming safety, which I independently confirmed holds.

## 4. What was done well

- Explicitly re-examined the codebase for the exact bug class the last review caught (`MOVE`'s rounding mismatch) before claiming this operation is safe, and the reasoning given holds up under direct inspection of `radius_px`'s type and `set_parameter`'s rounding — this is the right way to respond to prior review feedback, not just patching the flagged instance but checking siblings for the same class of mistake.
- Correctly reasoned about a two-pass separable kernel's *combined* support region rather than naively treating each pass independently — I verified this is actually correct (both analytically and empirically against the real two-pass code), not just plausible-sounding.
- The bonus `chaining_blur_into_an_unmodified_invert_is_still_pixel_identical` test goes beyond this phase's own (lighter) acceptance criteria — Phase 2's spec only requires AC1/AC2, with no AC3-style chaining requirement, but the implementor added the same rigor Phase 1 used anyway.
- Diff confinement and unchanged-`execute()` claims both check out exactly as stated.

## 5. Recommendation

**✅ Approve**
