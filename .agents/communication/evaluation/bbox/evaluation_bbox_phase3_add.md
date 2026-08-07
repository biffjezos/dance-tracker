# Evaluation: Phase 3 — ADD consumes bboxes (closing the Phase 3 gap, RFC-002)

**Commit:** `32f6a44` (code) + `6d84e15` (report) on `claude/bbox-phase-3-add`, merged to `dev` via PR #113 (`0b070ff`)
**Spec:** RFC-002 (`.agents/communication/rfc/RFC002addoperationbboxphase3.md`)
**Report:** `.agents/communication/implementation_reports/Phase3_add_report.md`

## 0. Process note — this was already merged to `dev` without a recorded evaluation

Before reviewing the code: `add.rs` is not a pending diff, it's already on `dev` (PR #113, merged before PR #114's WebGPU Phase 0 work landed on top of it). No `evaluation_bbox_phase3_add.md` existed anywhere in `.agents/communication/evaluation/` prior to this one — every other Phase 3 operation (blur, invert, shuffle, chromakey, ghost, screen, subtract, hue_key) has a matching evaluation on record; `add` didn't. This is the same shape of problem RFC-001 called out for the WebGPU commits (code reaching `dev` without going through the evaluator) — smaller blast radius here since the change is small and, per my review below, correct, but it's still a process gap worth flagging to Management: this evaluation is happening *after* the merge, not before it, and had no chance to block anything if it had found a real defect.

## 1. The build-environment claim — attempted independent verification, could not reach parity with the report

The report says `cargo test --lib add::` gave 10/10 (6 pre-existing + 4 new) and the full suite gave 287/0 failed, run against `dev` tip `4cab051` (pre-WebGPU-Phase-0, post-RFC-001 — genuinely a clean, `wgpu`-free base at that point).

I could not reproduce this. Per `ENVIRONMENT_DIAGNOSTICS.md`'s Steps 1-2, before concluding "environment restriction" I checked the proxy directly rather than inferring from one failed build:

```
curl -sS "$HTTPS_PROXY/__agentproxy/status"   # noProxy lists index.crates.io explicitly
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://index.crates.io/config.json
  -> HTTP 403, body: "Host not in allowlist: index.crates.io. Add this host to your network egress settings to allow access."
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://static.crates.io/crates/inventory/inventory-0.3.15.crate
  -> curl: (56) CONNECT tunnel failed, response 403
$HTTPS_PROXY/__agentproxy/status .recentRelayFailures -> one entry, static.crates.io:443, connect_rejected/403
```

This is a **stricter restriction than the one already on record** (`RFI_response_wgpu_sandbox_network.md`, confirmed restricted for `static.crates.io` only, with `index.crates.io` explicitly reachable/200 there). In *this* sandbox, `index.crates.io` itself now 403s despite being named in the proxy's own `noProxy` list — a different failure shape (an egress-allowlist rejection, not a proxy CONNECT rejection; it isn't even in `recentRelayFailures`, which only tracks proxy-relayed CONNECTs). I tried the same workaround the `SCREEN` evaluation used (apply the diff onto the last pre-`wgpu` commit, `0b070ff`, and build there, since dependency resolution alone needs only `index.crates.io`) — same 403, because in this sandbox even the index is unreachable, not just the crate-body host. There is no local cargo registry cache and no vendored dependency tree (`ui/vendor` exists for something else; nothing under `engine/` or `.cargo/`) to fall back on.

**I am not treating this as a code defect** — it's a session-level network policy question, same category as the already-open wgpu restriction, just a broader instance of it. Per `ENVIRONMENT_DIAGNOSTICS.md`: "code review and delivery must record the affected acceptance criteria as unverified, not passing." Filing a Notification to Management alongside this evaluation with the raw output above, since this is a new/different observation (even `index.crates.io`, not just `static.crates.io`) worth having on record.

**Acceptance Criterion #1 (RFC-002: `cargo build`/`cargo test` succeed, 4 new tests pass) → UNVERIFIED, not confirmed passing.** Everything below is a manual/static review, not a build-verified one.

## 2. Manual verification against RFC-002 and the established pattern

- **Diff scope** (Acceptance Criterion #4): `git diff 4cab051...0b070ff --stat` → exactly `engine/src/operations/compose/add.rs` and the implementation report. ✅ Matches the report's own claim and RFC-002's constraint.
- **Unmasked path unchanged** (Acceptance Criterion #2): `add_pixels` is untouched; `execute()`'s `else` branch still calls `Self::add_pixels(&first_image.pixels, &second_image.pixels)` directly, byte-for-byte as before. ✅
- **Is the union genuinely required (not just a defensible choice)?** Checked the algebra directly: `ADD`'s per-channel op is `a + b`. Setting `a = 0`: `add(0, b) = b` — adding black to real `Background` content reproduces that content unchanged (the existing `adding_black_is_identity` test already establishes this). So `ADD`, like `SCREEN`, is non-default whenever *either* input alone is non-default — the natural box must be the **union** of `Foreground`'s and `Background`'s boxes, not an intersection (would silently drop real content confined to only one side) and not either box alone (same failure). This is exactly RFC-002's stated reasoning, and it's correct. ✅
- **`add_single_pixel` vs. `add_pixels` equivalence:** identical per-channel `a[idx+c] + b[idx+c]` at one index vs. the whole-buffer loop — no discrepancy, no windowing/edge-index risk (uses the same `((y*width+x)*4)` indexing convention as `screen_single_pixel`). ✅
- **`compute_within_bbox`/`apply_mask` usage:** read `compute_within_bbox` directly (`engine/src/graphics/mask.rs`) — it copies `original` (here, `first_image.pixels`, i.e. Foreground) for every pixel outside `work_area`, then overwrites only the in-box pixels via `compute`. `apply_mask`'s first argument is likewise `first_image.pixels` (Foreground). Matches the established convention every other compose op (`SCREEN`, `SUBTRACT`) uses Foreground as the masked-out substitute. ✅
- **No `output_bbox()` override** — confirmed, consistent with RFC-002 and `SCREEN`/`SUBTRACT` (compose ops only participate in consume, not report, this round). ✅
- **The load-bearing test, traced by hand:** `consume_equivalence_requires_the_union_not_the_intersection_of_foreground_and_background_boxes` — `Foreground` reports `Rect::empty()`, `Background` reports `[3,7)` on a 10-wide frame, `Mask` reports full. `union(empty, [3,7)) = [3,7)` (per `bbox.rs`'s own `union_with_an_empty_operand_returns_the_other_operand_unchanged`), `intersect([3,7), full) = [3,7)`. Restricted compute covers `x ∈ [3,7)` only; outside that range `Foreground` is all-zero and `Background` is also zero there (test sets it up that way), so full-frame and restricted results agree everywhere, and the pinned assertion at `x=4` (`[200,150,100,255]`) confirms `Background`'s real content was actually summed in, not left as untouched Foreground zero. This is precisely the scenario that would fail under either wrong implementation (intersection-only or single-box). ✅ Correct by hand-trace; matches the same shape of test `SCREEN`'s evaluation already validated for the identical union requirement.
- **`a_smaller_mask_bbox_computes_strictly_fewer_pixels...`** — uses `reset_pixels_computed`/`take_pixels_computed`, correctly wired against `compute_within_bbox`'s own instrumentation (verified that mechanism directly in `mask.rs`). Logic checks out: 1×1 mask box → 1 computed pixel, full frame → 16, on a 4×4 image. ✅
- **`checkerboard_resize_move_geometric_mask_end_to_end...`** — same graph-based on/off pattern (`RenderExecutor` vs. direct `execute()`) already used by every other Phase 3 op's equivalent test; wiring (`Foreground`/`Background`/`Mask` inputs, `RESIZE`→`MOVE` for a geometric mask) looks correct and structurally identical to `screen.rs`'s version. ✅ by pattern match, not independently executed.
- **Test count claim:** counted directly in the diff — 6 pre-existing (`adding_above_255_is_left_out_of_gamut_not_clamped`, `a_sum_that_stays_in_gamut_round_trips_through_clamp_unchanged`, `adding_black_is_identity`, `chaining_add_into_add_accepts_the_out_of_gamut_float_image_input`, `a_zero_alpha_mask_passes_through_foreground_unadded`, `a_full_alpha_mask_adds_exactly_as_unmasked`) + 4 new = 10, matches the report's claimed `10/10`. Could not independently execute them (see §1).

## 3. Issues

None found in `add.rs` itself — design and test logic both check out against RFC-002 and the established `SCREEN`/`SUBTRACT` pattern, by hand-trace and static reading.

**Process findings (not code defects):**
- **Blocking for process, not for this specific diff:** the change reached `dev` (PR #113) with no evaluation on record beforehand. Recommend Management confirm whether this was an intentional exception or a gap in enforcement, since nothing currently prevents a repeat.
- The build-environment restriction (§1) is broader than the one already tracked (`RFIwgpusandboxnetworkcheck.md`) — this sandbox blocks `index.crates.io` too, not just `static.crates.io`. Filing a Notification with Management alongside this evaluation.

## 4. What was done well

- Correctly identified `ADD` shares `SCREEN`'s exact failure mode (non-zero-preserving on either input alone → needs a union) rather than defaulting to the simpler single-box pattern most other Phase 3 ops used, and named the reasoning explicitly rather than by analogy alone.
- The load-bearing test isolates the union requirement as precisely as `SCREEN`'s did — an empty box on one side, real content confined to the other, with a direct pixel-value assertion (not just equality) proving the content actually got summed in.
- Diff scope is minimal and exactly as RFC-002 specified: one file plus the report, unmasked path untouched.
- Transparent about the build-environment complication in the report itself (the "Build-environment note" section), even though in this case they reported *no* issue was hit (dev's tip at the time was clean) — consistent with prior rounds' practice of always addressing it explicitly rather than omitting it.

## 5. Recommendation

**✅ Approve the code itself** — `ADD`'s bbox-consumption logic is correct by design review and hand-traced test logic, consistent with RFC-002 and the `SCREEN`/`SUBTRACT` precedent. This is a retroactive approval since the change is already merged to `dev`; nothing here would have blocked the merge had it been evaluated beforehand.

**⚠️ Two items for Management, not for the Software Developer:**
1. Confirm intent behind `add.rs` reaching `dev` without a prior evaluation (process gap).
2. This sandbox's network restriction is stricter than previously documented (blocks `index.crates.io`, not just `static.crates.io`) — Acceptance Criterion #1 remains formally unverified by me; see the accompanying Notification.
