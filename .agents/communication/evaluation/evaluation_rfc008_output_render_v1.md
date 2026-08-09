# Evaluation: RFC-008 — OUTPUT menu + RENDER node v1 (RFI-RFC-008-READY)

**Branch:** `claude/dev-session-plw4fq` (not yet merged), commit `c6aa428`
**Spec:** SPEC-OUTPUT-RENDER-V1
**RFC:** `RFC008outputrenderv1.md` (Software Architect, new feature, medium priority)
**Report:** `.agents/communication/implementation_reports/RFC008_output_render_v1_report.md`
**Files touched:** `engine/src/compositor/metadata.rs`, `engine/src/operations/mod.rs`,
new `engine/src/operations/output/{mod,render}.rs`, `ui/index.html`,
`ui/scripts/engine/nodeEditContexts.js`, `ui/scripts/features/output.js`,
`ui/templates/partials/statusbar.hbs`. Confirmed via `git diff origin/dev..HEAD --stat` —
matches RFC-008's scope exactly, nothing extra touched.

## 0. Build-verification status — same restriction, independently reconfirmed

```
cd engine && cargo check
  -> download of config.json failed ... 403, "Host not in allowlist: index.crates.io"
```

Same as RFC-005/006/007's precedent. **AC6 (`cargo build`/`cargo test` succeed) is
UNVERIFIED, not confirmed passing.** No JS test harness exists in this repo (confirmed by
the spec itself), so the UI-side changes were never going to be automated-test-verifiable
either — everything below is manual trace against the real source, cross-checked against
existing precedent patterns in the same files.

## 1. Scope

Diff matches RFC-008's four required changes exactly, one file each, nothing extra. ✅

## 2. `OperationCategory::Output` (metadata.rs)

New variant added alongside the existing seven, `as_str() => "output"` — same shape as
every sibling variant, no special-casing. ✅

## 3. `Render` operation — traced against SPEC's exact field requirements

`descriptor()`: `id: "render"`, `menu: "OUTPUT"`, `label: "RENDER"`, `action: None`,
`ui_action: None`, `create_node: Some("render")`, `submenu: None` — matches the spec's
literal field list. `metadata()`: `category: Output`, `inputs: vec![]`,
`outputs: vec![OutputKind::Boolean]` — `Boolean` is a real, existing `OutputKind` variant
(confirmed in `metadata.rs`), a reasonable "least-ceremony placeholder" choice per the
spec's own permissive wording ("whatever satisfies the trait with the least ceremony").
`parameters()` uses the trait default (empty) — v1 correctly has none. `execute()`
returns `Ok(vec![Value::Boolean(true)])`, a harmless stub — `Value::Boolean(bool)` is a
real variant. Module registration (`pub mod output;` in `operations/mod.rs`,
`output/mod.rs`'s `pub mod render; pub use render::Render;`) mirrors every sibling module
exactly. `inventory::submit!` block present, same shape as every other operation. ✅

## 4. `supports_edit()` override — verified necessary and correctly implemented, not just plausible

Checked the trait's actual default rather than accepting the developer's claim on faith:

```rust
// compositor/operations.rs
fn supports_edit(&self) -> bool {
    !self.parameters().is_empty() || self.metadata().input_count() > 0
}
```

`Render` has empty `parameters()` and zero `inputs`, so the default resolves to `false`.
Traced the actual gating chain: `ui/scripts/engine/menu.js`'s `renderEditButton()` checks
`selectionState.supportsEdit()` (`nodeSelection.js`) → `wasmApp.node_supports_edit(nodeId)`
→ `engine/src/app.rs:237`'s `node_supports_edit`, which calls
`op.supports_edit()` directly on the graph node. Without the override, RENDER's EDIT
button — and therefore its only reachable EXECUTE trigger — would never render. The
override is not just plausible reasoning, it's the only way AC3/AC4 can hold given the
trait's real default. Correctly implemented (`fn supports_edit(&self) -> bool { true }`),
matches the report's own claim exactly. ✅ Confirms the developer's own request to double-check this.

## 5. Statusbar index-shift arithmetic — hand-traced, not accepted on faith

Enumerated `<footer class="statusbar">`'s element children by hand from the current
`statusbar.hbs`/`index.html` (both, since `index.html` was hand-edited — see §6):

Before this diff: `[0] STATUS, [1] FPS, [2] TYPE, [3] REC, [4] output-resolution,
[5] gamut-warning]`. After removing `REC`: `[0] STATUS, [1] FPS, [2] TYPE,
[3] output-resolution, [4] gamut-warning]`. The diff changes `output.js`'s
`applyOutputSize()` from `bar.children[4]` to `bar.children[3]` — matches the hand-count
exactly. `toggleRecord`'s own now-dead `bar.children[3]` REC-text writes (both start and
stop branches) are removed entirely, correctly, since that field no longer exists — the
recorder start/stop logic itself (`recorder.start()`/`recorder.stop()`) is untouched, per
the spec's explicit constraint. ✅ Confirms the developer's own request to double-check
this arithmetic.

## 6. `ui/index.html` hand-edit — diffed directly, not assumed

Compared the `ui/index.html` diff against the `statusbar.hbs` diff line-by-line: both
remove the exact same `<span> REC: OFF</span>` line, in the same position relative to
`TYPE`/`output-resolution`, with no other difference between the two files' changed
region. `grep -n "REC" ui/index.html` returns nothing — no stray leftover reference.
Matches the report's claim that the hand-edit is faithful to what a real
`npm run build` would have produced. ✅ Confirms the developer's own request to verify
this specifically rather than assume it.

## 7. `nodeEditContexts.js` — new bespoke context registration

`renderRenderEditContext(menuManager, nodeEntry)` calls `renderGenericEditContext`
first (same two-argument signature as its real declaration, confirmed at line 138), then
uses `startParamRow(menuManager)` — a real, existing helper (line 338) already used by
every other parameter-row-rendering path in this file, not a new ad hoc DOM pattern — to
append one `EXECUTE` button dispatching `toggleRecord`, the same event
`output.js`'s existing listener already handles. Registered via
`nodeEditContextRegistry.register("render", renderRenderEditContext)`, alongside the
existing `registerDefault(...)` call, mirroring `renderImageEditContext`'s real precedent
(confirmed at line 120) as the spec requires. ✅

## 8. Menu system — confirmed no `menu.js` change needed, as claimed

`CATEGORY_ORDER` (`menu.js:36`) already includes `"OUTPUT"` — confirmed directly, and
`git diff --stat` confirms `menu.js` has zero changes in this diff. The spec's own claim
("no menu.js change is needed... registering an operation with menu: OUTPUT is
sufficient") holds. ✅ (AC1)

## 9. Test suite — standard shape, matches sibling zero-input operations

Five new tests: descriptor sanity, metadata (category + empty inputs), empty parameters,
`supports_edit()` true, execute-stub value, plus a graph-validity test
(`render_in_graph_is_valid`) whose shape (`Graph::new`, `add_node`, `validate()`,
`RenderExecutor::execute`) directly mirrors the existing zero-input-operation pattern in
`animate/square.rs`'s and `animate/sine.rs`'s own `*_in_graph_is_valid` tests — not a
novel or untested API usage. Brace balance on `render.rs` confirmed even (28/28). ✅

## Decision

**Approve.** All three items the developer specifically flagged for extra scrutiny
(`supports_edit()` override, statusbar index arithmetic, the hand-edited `index.html`)
were independently re-derived and re-traced, not accepted on the report's word — all
three hold up exactly as claimed. Scope matches RFC-008/SPEC-OUTPUT-RENDER-V1 precisely,
no out-of-scope settings (FILE_NAME/FORMAT/RESOLUTION/FPS/COMPRESSOR/FROM-TO) introduced,
`recorder.js` untouched as required. `cargo build`/`cargo test` and a live click-through
are both unverified (environment/toolchain limitations, not defects) — no JS test harness
exists in this repo at all, so that gap is structural to the project, not something this
implementation could have closed. No critical defects found; nothing blocking merge.
