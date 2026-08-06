# Specification: Fold `SystemMenu` into the existing operation inventory

**Priority:** Low — only matters once something actually needs a non-node-creating menu entry with a UI (the deferred COMPUTE MODE switch is the motivating example, but this spec doesn't build that; it just makes sure the *next* thing that needs this shape doesn't reintroduce the duplication). Safe to sequence well after RFC-001/RFC-002 and the WebGPU spec's Phase 0-1.

## Background

`compositor/system.rs`/`compositor/system_inventory.rs` introduced a second, parallel registration system — `SystemMenuDescriptor`, `SystemMenu`, `SystemMenuInfo`, its own `inventory::collect!`/`OnceLock` — structurally a near-duplicate of the existing `operations::inventory` (`OperationInfo`, the same `OnceLock`+`inventory::collect!` shape, just producing a different output type). Its only registrant, `compute_mode`, was removed by RFC-001 along with the rest of the broken GPU attempt, so `SystemMenu::descriptors()` currently returns an empty `Vec` — harmless, but pointless: a whole second inventory system with nothing in it.

The good news, checked directly against `menu.js`: **the menu *rendering* was never actually duplicated.** `systemMenus` and `operations` are concatenated into one array and flow through the exact same `renderOperationList()`/`renderOperationButtons()` — same category grouping, same submenu handling. `NODES` is the only genuine special case, and it predates this work entirely (it's how you browse/edit existing graph nodes — categorically different from "things you can add," nothing to consolidate there). So this spec is narrowly about the *registration* layer, not the UI layer.

`OperationDescriptor` already has an `action: Option<&'static str>` field (documented as "direct action") sitting alongside `ui_action`, which `video`/`image`/`camera` sources already use for exactly the "clicking this doesn't create a node, it does something" shape (`ui_action: Some("open_video_picker")`, etc.). `SystemMenuDescriptor` reusing `OperationDescriptor` wholesale was the right instinct — it should have gone one step further and reused the *registration* mechanism too.

## Design

**Remove:** `compositor/system.rs`, `compositor/system_inventory.rs`, `App.system_menus`, `App::get_system_menus()`, and the JS-side `systemMenus`/`systemMenusLoaded` wiring in `ui/scripts/core/wasm.js` and `menu.js` (the `...this.systemMenus` spread in `renderOperationList()`'s `menuEntries` becomes just `this.operations`; `renderCategoryButtons()` drops its separate `this.systemMenus.forEach(...)` loop). Since real operations already carry everything a "system menu entry" needs (`OperationDescriptor` with `create_node: None`, `action`/`ui_action`, `parameters()`), there's nothing left for a parallel type to do.

**Going forward: a non-node-creating setting is just a normal `Operation`.** Register it through the one existing inventory (`operations::inventory::OperationInfo`), same as everything else. Its `metadata()` declares `inputs: vec![]`/`outputs: vec![]` (it doesn't participate in the graph at all — it's not wired to anything, never `execute()`d as part of a render), its `descriptor()` sets `create_node: None` and `menu`/`submenu` for placement (`"PROJECT"`/`"SETTINGS"`, matching what `compute_mode` already used). This needs one small new concept the current `Operation` trait doesn't have a home for yet: **a way to read/write such an operation's own parameter value without a graph node id**, since `get_parameter`/`set_parameter` today are only ever reached via a real `NodeId` (`node_inputs`/`update_node_parameter` etc., all keyed by node). Two ways to close that gap, in order of preference:

1. **A small, fixed registry of "singleton" operations** the `App` owns one live instance of (distinct from graph nodes), addressed by `descriptor().id` instead of a `NodeId` — two new thin WASM bindings, `get_singleton_parameter(id: &str, name: &str)` / `set_singleton_parameter(id: &str, name: &str, value)`, that look the operation instance up by id and call its existing `get_parameter`/`set_parameter` exactly as today. No change to the `Operation` trait itself — it's purely about *where* the instance lives and *how* it's addressed from JS.
2. (Rejected) Giving every "system" operation a real, permanently-present, un-removable, unwired `NodeId` in the graph just so the existing node-parameter plumbing works unmodified. Simpler on the Rust side, but it's a lie about what a node is (a real graph slot for something that's never rendered, never wired, never deletable) — avoid it; it would also need its own carve-outs in graph validation/node-selector UI to hide it from "real" node lists.

**On the JS side**, `renderOperationButtons()` needs to do one more thing than it does today: when an operation has `create_node: None` but a non-empty `parameters()`, render its parameter row(s) inline (reusing the exact stepper-rendering helpers `nodeEditContexts.js` already has for real node parameters) instead of just dispatching a bare `action` event on click. This is the piece that was actually missing for `compute_mode` before — the descriptor and parameter existed, but nothing ever rendered the parameter itself. Reuse, don't reinvent: the stepper/enum-selector rendering code already exists for graph-node parameters; this just needs to call it against the singleton-parameter bindings above instead of `node_inputs`/node-keyed calls.

## Acceptance criteria

1. `grep -r "SystemMenu\|system_inventory" engine/src ui/scripts` returns zero matches.
2. `cargo build`/`cargo test` and the full existing suite pass unchanged.
3. The `PROJECT` menu category doesn't appear (nothing registered under it yet — correct, matches "only display what actually exists").
4. The singleton-operation registration path (item 1 above) is implemented and has at least one real Rust test proving get/set round-trips correctly by id, even with nothing wired up to it in the UI yet — this de-risks it for whenever the WebGPU spec's deferred COMPUTE MODE work actually needs it, rather than leaving it as an untested design on paper.
5. No graph-side change: `NodeId`, `Graph::add_node`, node validation, and the node-selector UI are completely untouched by this — singleton operations never enter the graph.

## Out of scope

- Actually building the COMPUTE MODE UI (parameters, the CPU/GPU/AUTO enum stepper, wiring it to `Context.gpu`/backend selection) — that's real future work, deferred by the WebGPU specification itself ("last priority"). This spec only makes sure that whenever it happens, it has a clean, non-duplicated place to register.
