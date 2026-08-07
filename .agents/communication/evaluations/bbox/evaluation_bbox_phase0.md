# Evaluation: Phase 0 — Bounding-box awareness foundation

**Commit:** bcf895d on `claude/bbox-phase-0`
**Spec:** `1_bboxawarenessspec.md` (Phase 0 section), against `BBOX_CONVENTIONS.md`

## 1. Summary of the change

Lands the zero-behavior-change scaffolding for bbox-awareness: a new `Rect` type (`compositor/bbox.rs`), a defaulted `Operation::output_bbox` trait method (full-frame default), a new `Context.input_bboxes` field, `find_bbox` next to `find_input`, and both executors (`RenderExecutor`, `PreviewExecutor`) threading `(Value, Rect)` pairs through their private recursion instead of bare `Value`. No operation file changes, no `execute()` changes, no public `Execute::execute()` signature change.

## 2. Verification against requirements

I read the actual Phase 0 spec (not just the implementor's own summary of it), checked the real diff against it file-by-file, built and ran the test suite myself, and independently re-derived the pixel math of the new integration test rather than trusting its comments.

- **`bbox.rs`:** `Rect { x0,y0,x1,y1 }` with `full`/`empty`/`is_empty`/`intersect`/`union`/`grow`, signed `i32` coordinates, `union`'s empty-operand-is-ignored behavior, `grow`'s no-clamp contract — matches `BBOX_CONVENTIONS.md`'s type-system section and the Phase 0 file list exactly. 12 unit tests present, including the two edge cases the spec explicitly calls for (empty-operand `union`, grow-then-clamp-by-caller). ✅
- **`input.rs`:** `find_bbox(bboxes: &[(Input, Rect)], key: Input) -> Option<Rect>` — same shape as `find_input`, as specified. ✅
- **`operations.rs`:** `Operation::output_bbox` signature is byte-for-byte the one both the spec and `BBOX_CONVENTIONS.md` give, defaulted to `Rect::full(ctx.meta.width, ctx.meta.height)`. Test `the_default_output_bbox_is_exactly_full_frame` covers AC2. ✅
- **`context.rs`:** `input_bboxes: Vec<(Input, Rect)>` added; `Context` was already `#[derive(Clone, Default)]`, confirmed directly — so the report's claim that no other construction site needed changes holds. ✅
- **Executors:** Diffed both `render.rs` and `preview.rs` in full. Both thread `(Value, Rect)` through their recursions, build `input_bboxes` from resolved children, construct a per-node `Context` via `{ input_bboxes: ..., ..ctx.clone() }` immediately before `execute()`, then call `output_bbox()` afterward — matching the spec's prescribed shape in both files, including `RenderExecutor`'s cache (`CachedNode` gains `bbox: Rect`) and both `evaluate`/`evaluate_profiled` paths. Public `Execute::execute()` still returns bare `Vec<Value>` in both. ✅
- **No operation file touched:** confirmed via `git show --name-only` (zero files under `operations/`) and a direct grep for `input_bboxes`/`output_bbox` across `operations/` on the actual commit — zero hits, so no operation reads or writes bbox data yet. ✅
- **AC1 (diff confined to `compositor/`, zero test edits elsewhere):** `git diff --stat` against the parent commit shows exactly 7 files, all under `engine/src/compositor/`. ✅
- **AC2/AC3 (unit tests):** present and correct, as above. ✅
- **AC4 (byte-identical integration test):** `phase_0_bbox_threading_does_not_change_a_real_multi_node_graphs_output` wires a real `chromakey → add` chain through the actual `Graph`/`RenderExecutor`. I independently re-derived the expected output rather than trusting the test's comments: `ChromaKey::new()` defaults to `key_color = (0,1,0)` and `threshold = 0.3`; a pure-green `(0,255,0,255)` source is at distance 0 from the key color, so its alpha is zeroed but RGB passes through unchanged (`key_pixels` only ever touches channel 3). `Add::add_pixels` sums all 4 channels unconditionally with no alpha-aware blending. So `(0,1,0,0) + (10/255,20/255,30/255,1)` → `R=10/255, G=1.0 (clamped), B=30/255, A=1.0` → `[10,255,30,255]` after clamping to `u8`. This matches the test's assertion exactly — the test is genuinely correct, not just self-consistent. ✅
- **Build/test:** ran `cargo build` and `cargo test` myself on the actual branch — 228 passed, 0 failed, only the two pre-existing warnings (`Point2D`/`Center` dead code, predating this change). Matches the report exactly. ✅

## 3. Issues

**Minor:**
- AC4 reads "a render/**preview** integration test confirms output is byte-identical... for at least one existing multi-node graph." Only `RenderExecutor` got a new pixel-identity test; `PreviewExecutor`'s identical threading changes (`evaluate_unmemoized`/`evaluate_memoized`) are only covered by the pre-existing stub-based tests, which check memoization call-counts, not actual pixel output. The two executors' changes are structurally near-identical copy-paste of the same three-line pattern (build `input_bboxes`, clone `Context` with override, call `output_bbox` after `execute`), so the residual risk is low — but it's still true that the one thing this phase exists to prove ("zero behavior change") is verified end-to-end for one of the two executors and not the other. A cheap fix: mirror the same `chromakey → add` graph test using `PreviewExecutor::execute` (both `memoize: true` and `false`) alongside the existing one. This is a suggestion, not a blocker — the spec's AC4 wording ("at least one... graph") is satisfied literally, and `CLAUDE.md`'s "don't over-invest" guidance cuts against demanding a second near-duplicate test for a mechanically identical code path.

No blocking or major issues found.

## 4. What was done well

- The integration test's math checks out under independent re-derivation, not just self-consistency — it genuinely proves the threading is inert, using a real `Graph`/`RenderExecutor`/multi-node chain rather than a synthetic stand-in.
- Precise adherence to the "reporting vs. consuming" split from `BBOX_CONVENTIONS.md`: nothing in `operations/` was touched, and a direct grep confirms no operation reads or writes the new bbox plumbing yet — this phase really is inert, not just claimed to be.
- Diff confinement is exact: all 7 changed files live under `compositor/`, matching AC1's explicit "if any existing operation test needed editing to compile, something touched more than it should have" bar.
- `Context.input_bboxes` reuses the existing `#[derive(Default)]`/`Clone` pattern instead of threading a new `execute()` parameter through every call site — verified this claim directly rather than taking it on faith, and it holds.
- The `CachedNode`/memo `HashMap` correctly carries `bbox` alongside `value` in lockstep everywhere, so the cross-tick cache can't return a stale/mismatched pairing.

## 5. Recommendation

**⚠️ Approve with minor comments** — ready to merge as-is; the one minor note (add a `PreviewExecutor` pixel-identity test analogous to the `RenderExecutor` one) is a nice-to-have for symmetry, not a blocker, since the spec's literal AC4 is satisfied and the two executors' changes are mechanically identical.
