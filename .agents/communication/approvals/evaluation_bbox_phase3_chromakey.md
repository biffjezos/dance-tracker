# Evaluation: Phase 3 — CHROMA KEY consumes bboxes (fourth operation)

**Commit:** 657db5d on `claude/bbox-phase-3-chromakey`
**Spec:** `1_bboxawarenessspec.md`, Phase 3 section (still held — no resend needed)

## 1. Summary of the change

`CHROMA KEY`'s masked path restricts computation to `intersect(SOURCE's box, MASK's box)` — no growth, since `key_pixels`/`key_single_pixel` read only the pixel they write. No `output_bbox()` override, correctly excluding the out-of-scope content-derived (keyed-out-alpha) tightening reserved for Phase 4.

## 2. Verification against requirements

Same scrutiny as the prior three operations, with particular attention here since `CHROMA KEY` is explicitly the operation `BBOX_CONVENTIONS.md`/`PARKED_WORK.md` calls out as having a *separate*, harder, deliberately-excluded problem (content-derived boxes from its own keyed-out alpha) — worth checking the implementation didn't blur that line.

- **Is `CHROMA KEY` genuinely zero-preserving, for any `KEY_COLOR`/`THRESHOLD`?** Read `key_pixels` directly: RGB is copied through unconditionally (`target[0..3] = source[0..3]`), and alpha is `0.0` if `distance <= threshold`, else `source[3]` — never a third case, no arithmetic that could produce a nonzero value from an all-zero input regardless of `key_color`/`threshold` (the `if` branch gives `0.0` either way when `source[3]` is already `0.0`). This holds unconditionally, not just for the default green key. ✅
- **`key_single_pixel` vs. `key_pixels` equivalence:** identical formula, operating on one index instead of the whole buffer — no window, so (unlike `BLUR`) there's no floating-point-order or normalization risk to check for. ✅
- **Scope discipline — did this correctly stay out of Phase 4's territory?** Confirmed: no `output_bbox()` override was added, and the diff and report both explicitly note that deriving a box from `CHROMA KEY`'s own keyed-out alpha remains untouched. The masked-compute restriction here only uses `SOURCE`'s and `MASK`'s *already-reported* boxes (upstream metadata), never inspects `CHROMA KEY`'s own output to derive anything — correctly staying on the "consume" side of the reporting/consuming split for inputs only, not extending it to self-reporting. ✅
- **Diff scope:** confirmed via `git show --stat` — exactly `chromakey.rs`, one file. ✅
- **Build/test:** ran myself — 266 passed, 0 failed, matching the report exactly. ✅
- **Independent adversarial verification:** wrote my own brute-force probe — 6 `SOURCE` box positions × 6 `MASK` box positions × 4 `KEY_COLOR`/`THRESHOLD` configurations (default green, a generous near-black key, a tight white key, and a **zero-threshold exact-match-only** edge case), with source pixels randomly mixed between colours that do and don't key out under each config. **144 trials, 0 violations.** The zero-threshold case is worth calling out specifically — it's the tightest possible keying behavior (only an exact `key_color` match keys out) and still held up. ✅
- **Test comment accuracy:** the `consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box` test's comment matches the code exactly, and — like `SHUFFLE`'s round — deliberately mixes a keyed-out pixel (`[0,255,0,255]`, pure green) and a non-keyed one (`[10,20,30,255]`) *inside* the restricted region itself, which is a meaningfully stronger test than a uniform fill would be. ✅

## 3. Issues

None found. No blocking, major, minor, or nit findings.

## 4. What was done well

- Correctly distinguished "consuming `SOURCE`'s/`MASK`'s already-reported boxes" (in scope) from "deriving a new box from `CHROMA KEY`'s own keyed-out alpha" (out of scope, Phase 4) — this is the operation where that line is easiest to blur, and the implementation and report both stayed on the correct side of it without needing prompting.
- The zero-preservation proof is airtight and, unlike a plausible-sounding argument, doesn't depend on the specific `KEY_COLOR` — I independently confirmed this holds even at the threshold's own edge case (exact-match-only).
- Test data was deliberately constructed to mix keyed and non-keyed pixels within the restricted region itself, which is a more honest test than a uniform fill (a uniform fill could pass by accident even with a latent bug; a mix can't).
- Fourth operation in a row with correct diff confinement, matching build/test counts, and an accurate, code-consistent set of comments.

## 5. Recommendation

**✅ Approve**
