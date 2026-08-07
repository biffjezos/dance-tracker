# Evaluation: Phase 3 — SCREEN consumes bboxes (sixth operation)

**Commit:** 3611a45 on `claude/bbox-phase-3-screen`
**Spec:** `1_bboxawarenessspec.md`, Phase 3 section (still held — no resend needed)

## 0. The build-environment claim — independently verified, not just trusted

The report flags something unusual: `dev`'s real tip now carries unrelated, in-flight GPU-backend work (`wgpu`/`pollster`/`bytemuck`) that fails to build in this sandbox, and says they worked around it by verifying on a pre-GPU-work base and hand-patching onto real `dev` for the PR itself, without being able to re-run `cargo test` against the exact final commit.

I did not take this on faith:
- Confirmed the GPU work is real and already on `dev` (commits like `biffjezos: comput backend`, `debug webgpu`, `wgpu = "30.0.0"` in `Cargo.toml`) — predates this bbox effort entirely, unrelated to any implementor/evaluator activity.
- Tried building the actual PR branch myself: it fails identically, `error: failed to download from https://static.crates.io/... [56] CONNECT tunnel failed, response 403`. Checked the proxy status directly (`$HTTPS_PROXY/__agentproxy/status`) — `static.crates.io` is explicitly policy-denied in this sandbox's `noProxy` list (unlike `index.crates.io`, which is allowed). This is a genuine, deliberate sandbox restriction, not a fluke — I hit the exact same wall.
- Rather than accept "it applied byte-for-byte identically to a version that passed" as sufficient, I reproduced their verification path independently: extracted the `screen.rs` diff against real `dev` (`git diff origin/dev...HEAD`, confirmed exactly two files — `screen.rs` and the report), applied it onto the pre-GPU-work commit myself (the merged `GHOST` branch tip, `6650a40`), and ran `cargo build`/`cargo test` there myself. **275 passed, 0 failed** — matches the report exactly, on a build I ran myself, not one I was told about.

This is a real, environment-level blocker (not specific to this PR) that will affect every future evaluation in this repo until either the network policy allows `static.crates.io`, the GPU dependency is made optional, or vendored. Worth surfacing to you directly, separate from this code review — it's not something the implementor or I can fix from here.

## 1. Summary of the change

`SCREEN`'s masked path restricts computation to `intersect(union(Foreground's box, Background's box), Mask's box)` — a **union** of both inputs' boxes, not an intersection or a single input's box like every prior operation used.

## 2. Verification against requirements

- **Is the union genuinely required, not just a defensible choice?** Verified `screen_pixels`'s formula directly: `screen(a,b) = 1 - (1-a)(1-b)`. Setting `a=0`: `screen(0,b) = 1-(1)(1-b) = b` — screening black against real content reproduces that content unchanged, confirmed algebraically, matching the pre-existing `screening_with_black_is_identity` test. So `SCREEN` is non-default whenever *either* input is non-default — the natural box must be the union of both, not an intersection (which would silently drop whichever input's real content falls outside the other's box) and not either box alone (same failure, one-sided). ✅
- **`screen_single_pixel` vs. `screen_pixels` equivalence:** identical per-channel formula at one index vs. the whole buffer — no window, no risk class here. ✅
- **`apply_mask`/`compute_within_bbox` consistency:** both use `Foreground`'s raw pixels as the "original"/pass-through value, matching the established convention (unlike `GHOST`, which correctly needed a different substitute for a different reason). ✅
- **The report's own load-bearing test:** `consume_equivalence_requires_the_union_not_the_intersection_...` — read it directly: `Foreground` reports an *empty* box, `Background` carries the only real content in `[3,7)`. If the code used only `Foreground`'s box or the intersection, `work_area` would collapse to empty and silently skip screening in `Background`'s real content. This is exactly the right test to isolate the union requirement, and it passes. ✅
- **Independent adversarial verification, going further than the report's one pinned scenario:** wrote my own brute-force probe — 6 `Foreground` × 6 `Background` × 6 `Mask` box positions (216 combinations), each with independently randomized real content on *both* sides simultaneously (not just one side empty, as the report's own test does) — including disjoint, overlapping, and edge-touching box configurations. **216 trials, 0 violations**, run against the real code in a build I compiled myself. ✅
- **Diff scope:** confirmed via `git diff origin/dev...HEAD --stat` against the real current `dev` tip — exactly `screen.rs` (plus the report markdown). ✅
- **Build/test:** independently reproduced — 275 passed, 0 failed, matching the report's own number exactly, on my own build rather than their say-so.

## 3. Issues

None found in the `SCREEN` implementation itself. No blocking, major, minor, or nit findings.

**Process note (not a code defect):** the build-environment blocker described above is real and will recur on every subsequent phase until addressed at the repo/environment level. I'd flag this to whoever owns the `wgpu` work and this sandbox's network policy — not something for the bbox implementor to solve mid-phase, but worth resolving before it becomes a recurring tax on every future evaluation.

## 4. What was done well

- Correctly identified that `SCREEN` breaks the established pattern *differently* than `INVERT` did (not "isn't zero-preserving on its one relevant input," but "isn't zero-preserving on either input alone, needs a union of two") — this is a distinct failure mode from the ones the last two operations taught, and the report explicitly names the distinction rather than pattern-matching a prior fix.
- The load-bearing test is precisely targeted: an empty `Foreground` box with all real content on `Background`'s side is the single sharpest case for proving "union, not intersection, not either alone" — it would fail under either wrong implementation.
- Transparent, proactive disclosure of the build-environment problem rather than silently working around it and hoping it wasn't noticed — including exactly how they verified the PR branch still matches what was tested, which is what let me reproduce and confirm their verification path instead of having to start from scratch.
- Diff scope confirmed against the *real* `dev` tip (not a stale local branch), which is the correct base for that claim given the environment complication.

## 5. Recommendation

**✅ Approve** — the `SCREEN` change itself is correct and thoroughly verified (by both the implementor and independently by me, against a real compiled build). The build-environment issue is real but orthogonal to this PR's correctness; it doesn't block this specific merge, but it does need attention before the next phase's evaluation runs into the same wall.
