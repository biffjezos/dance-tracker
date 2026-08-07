# Evaluation: Phase 3 — BLUR consumes bboxes (first operation)

**Commit:** 2f28677 on `claude/bbox-phase-3-blur`
**Spec:** `1_bboxawarenessspec.md`, Phase 3 section (still held from earlier in this session — no need to resend)

## 1. Summary of the change

Lands the shared Phase 3 infrastructure (`compute_within_bbox` in `graphics/mask.rs`, a thread-local pixels-computed counter threaded into `ProfileEntry` via `RenderExecutor::execute_profiled`) and migrates `BLUR` as the first consumer: with a wired `MASK`, `execute()` restricts actual blur computation to `intersect(SOURCE's reported box, MASK's reported box)` instead of computing the full frame unconditionally.

## 2. Verification against requirements

Phase 3 is qualitatively different from Phases 1–2: it changes real runtime behavior (pixels actually computed), not just metadata. A bug here produces visibly wrong output, not a latent risk for later. I gave this the same brute-force/direct-reproduction treatment that caught the Phase 1 bug, and it caught something.

- **Diff scope:** confirmed via `git show --stat` — exactly the 5 files claimed (`mask.rs`, `graphics/mod.rs`, `profiling.rs`, `executors/render.rs`, `blur.rs`). ✅
- **Build/test:** ran myself on the pristine branch — 253 passed, 0 failed, matching the report exactly. ✅
- **`compute_within_bbox`:** matches the spec's signature and semantics exactly — starts from `original.to_vec()`, only overwrites pixels inside the (locally re-clamped) work area, records the computed count via a thread-local. ✅
- **Profiling plumbing:** `reset_pixels_computed()`/`take_pixels_computed()` bracket `execute()` in `evaluate_profiled` correctly; since evaluation is single-threaded recursion with each node's `execute()` a pure function of already-resolved input `Value`s (no re-entrant `execute()` calls), there's no risk of a child node's count leaking into a parent's — verified by reading the recursion structure directly. `Profile`'s `Display` extension is a clean, non-invasive addition. ✅
- **`blur_single_pixel` vs `blur_pixels` mathematical equivalence claim:** the report claims these are "mathematically identical... not an approximation." I wrote a brute-force test comparing them directly across 40 (dimension × radius) combinations with random pixel data — the two agree to within ~1e-7 per channel everywhere (tiny floating-point summation-order noise from computing the same real-valued quantity via a two-pass vs. single-pass reduction, well below any u8-quantization threshold). The claim holds. ✅

## 3. Issues

**Blocking:**

- **`BLUR::execute()`'s masked path violates the Phase 3 consume-equivalence invariant whenever `SOURCE` itself reports a non-full-frame box** (i.e., whenever `BLUR` is fed from an upstream operation that has adopted Phase 1/2 reporting — `RESIZE`, `MOVE`, or any future geometric op). The bug:

  ```rust
  let natural_box = find_bbox(&ctx.input_bboxes, Input::Source)
      .unwrap_or_else(|| Rect::full(source.width, source.height));
  ...
  let work_area = natural_box.intersect(&mask_box);
  ```

  This uses `SOURCE`'s *raw* reported box as one operand of the intersection. But `BLUR`'s own true content-spread extent — which this same file's Phase 2 `output_bbox()` already computes — is `SOURCE`'s box **grown by `radius_px`**, because a box blur pulls real neighboring content up to `radius_px` pixels beyond the source's own non-default region. By intersecting against the un-grown box, `work_area` can be strictly smaller than the region `BLUR` actually needs to compute, silently skipping real blur computation in the "penumbra" annulus between `SOURCE`'s box and its radius-grown extent — whenever `MASK`'s own weight is nonzero there.

  **Why none of the new tests catch it:** all three new Phase 3 tests (`consume_equivalence_...`, `a_smaller_mask_bbox_...`, `checkerboard_resize_move_...`) construct `ctx.input_bboxes` (or a graph) where `Input::Source`'s reported box is always `Rect::full` — only `Input::Mask`'s box is ever varied. When `SOURCE`'s box is already full-frame, `grow(full, r).intersect(full) == full == source_box.intersect(full)`, so the missing-growth bug is mathematically invisible in every test as written. This isn't a contrived gap — it's the exact scenario Phase 1/2 exist to enable (feeding `BLUR` from a `RESIZE`/`MOVE` chain), and it's also the direction the codebase is visibly heading: the graph-level integration test even *uses* `CHECKERBOARD → RESIZE → MOVE`, just wired to `MASK` instead of `SOURCE`.

  **Reproduction** (verified against the real code, not modeled): 10×1 frame, `SOURCE` pixels genuinely `[0,0,0,0]` outside `[3,7)` and real content `[100,100,100,255]` inside it (a valid precondition — the reported box must match where real content actually is, per `BBOX_CONVENTIONS.md`'s own invariant), `RADIUS=2`, `MASK` fully opaque everywhere. Comparing `execute()` with the real `SOURCE` box `[3,7)` against `execute()` with an empty `ctx.input_bboxes` (full-frame fallback — the "ground truth" pre-Phase-3 result):
  ```
  restricted (buggy):        [0,0,0,0] [0,0,0,0]  [0,0,0,0]  [0,0,0,0]  ... (pixels 0,1 left as raw transparent)
  unrestricted (correct):    [0,0,0,0] [25,25,25,64] [40,40,40,102] ...  (pixel 1 genuinely blurred, nonzero)
  ```
  Pixel `x=1` sits in the radius-2 penumbra just outside `[3,7)`; the correct computation blurs real neighboring content into it (`[25,25,25,64]`), but the buggy restricted path copies the original transparent value through unchanged. This is wrong, visible output — not merely a missed optimization.

  **Fix**, confirmed to resolve the violation (re-ran the reproduction test against the patched code — passes, and the full 253-test suite still passes on top of it): grow `natural_box` by `radius_px` before intersecting, mirroring the exact formula this file's own `output_bbox()` already uses:
  ```rust
  let natural_box = find_bbox(&ctx.input_bboxes, Input::Source)
      .unwrap_or_else(|| Rect::full(source.width, source.height))
      .grow(self.radius_px as i32)
      .intersect(&Rect::full(source.width, source.height));
  ```
  Worth going a step further than the minimal patch: since this is now the *second* place in the file computing "SOURCE's box grown by radius, clamped to frame" (the first being `output_bbox()` itself), consider factoring it into a small private helper both call, so `output_bbox()`'s reported metadata and `execute()`'s actual work area can never drift apart again the way they just did.

  **Regression test to add:** a variant of the existing `consume_equivalence_...` test where `Input::Source`'s reported box is genuinely sub-frame (not `Rect::full`) — the counter-example above is a ready-made template.

No other blocking issues. No major, minor, or nit findings beyond the one above.

## 4. What was done well

- The shared `compute_within_bbox` helper and profiling-counter plumbing are clean, match the spec's shape exactly, and correctly avoid leaking counts across sibling/child node evaluations in the recursive executor.
- The `blur_single_pixel`/`blur_pixels` equivalence claim is real and was worth making explicit — I independently confirmed it holds (to floating-point noise) across many configurations, not just the two pre-existing masked tests that happened to already pass.
- The instinct to add a graph-level integration test using the exact `CHECKERBOARD → RESIZE → MOVE` pipeline from the `MOVE` spec is the right one — it's just wired to the wrong input (`MASK` instead of `SOURCE`) to catch this particular bug, which is somewhat unlucky given how close it came to exercising the real scenario.
- Everything not touching the `work_area` computation itself (diff scope, `execute()` structure, instrumentation, `apply_mask` integration) is correct and well-tested.

## 5. Recommendation

**❌ Request changes** — the `work_area` computation must account for `BLUR`'s own radius-driven spread (grow before intersect), not just `SOURCE`'s raw reported box, before this can merge. The fix is small and localized; everything else in this phase is solid.
