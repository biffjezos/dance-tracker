# Evaluation: RFC-007 (critical) — App.gpu wasm-bindgen borrow lockup fix (RFI-RFC-007-READY)

**Branch:** `claude/dev-session-plw4fq` (not yet merged), commit `9138533`
**Spec:** SPEC-webgpu-compute-backend, "App boot" section (RFC-007's own correction)
**RFC:** `RFC007initgpuborrowlockup.md` (Software Architect, critical — Management-reported
total app lockup after a single WebGPU init failure)
**Report:** `.agents/communication/implementation_reports/RFC007_gpu_borrow_lockup_fix_report.md`
**File touched:** `engine/src/app.rs` only. Confirmed via `git diff origin/dev..HEAD --stat`.

## 0. Build/runtime-verification status — same limitation, one layer deeper

Independently re-attempted both checks the report says are unavailable:

```
cd engine && cargo check --target wasm32-unknown-unknown
  -> download of config.json failed ... 403, "Host not in allowlist: index.crates.io"
```

Confirms the network restriction blocks even a `check` (no full build needed) for this
target. The report's second, structural point also holds: `app.rs` is
`#![cfg(target_arch = "wasm32")]`-gated with zero existing tests, so it was never reachable
by native `cargo test` even before this change, and this repo has no committed wasm32
build/test harness. **This evaluation is a manual/static review of Rust source and
wasm-bindgen's documented object-borrow-guard semantics — it cannot observe the actual
runtime behavior in a browser.** Per the report's own recommendation and RFC-007's
critical severity, **Management or a session with real browser access should confirm
this fix in a live build before RFC-007 is considered fully closed**, regardless of the
approval below.

## 1. Scope

Diff is exactly `app.rs`: field type, `App::new()`, `init_gpu`, `context()`. Nothing else
touched. Matches RFC-007's stated scope. ✅

## 2. The core mechanism — independently traced, not just read

`App.gpu`: `Option<Arc<GpuState>>` → `Rc<RefCell<Option<Arc<GpuState>>>>`. ✅ (AC1, part 1)

`init_gpu`: `pub async fn init_gpu(&mut self) -> Result<(), JsValue>` →
`pub fn init_gpu(&self) -> js_sys::Promise`. This is the actual fix, and the reasoning
holds up under independent trace:

- The old signature's `async fn(&mut self)` desugars to a `Future` whose *type* captures
  `&mut self` for its entire lifetime — including every suspended `.await` point inside
  it (here, `GpuState::new().await`, itself awaiting `adapter.request_device(...)`).
  `wasm-bindgen` tracks a JS-visible "checked out" guard on the object for as long as any
  future capturing that borrow is still being polled. A raw JS exception (the observed
  `requestDevice`-on-null `TypeError`) escaping mid-poll bypasses the normal poll-return
  path that releases that guard on completion (`Ok` or a clean `Result::Err` both go
  through it fine) — the guard is left stuck, and every subsequent call to any method on
  the same object is refused ("recursive use... unsafe aliasing").
- The new shape moves all the awaiting into an inner `async move` block passed to
  `wasm_bindgen_futures::future_to_promise`, capturing only `gpu_cell` — a cloned `Rc`,
  a plain value, not a wasm-bindgen-tracked borrow of anything. `init_gpu` itself is no
  longer `async` — its own body is one `Rc::clone` and a call to `future_to_promise`, both
  synchronous. Whatever `wasm-bindgen`-level borrow guard exists for the synchronous
  `&self` call is checked out and released within that single call frame, **before**
  `GpuState::new()` ever starts polling — not held across any `.await`, because `init_gpu`
  itself has no `.await`. No matter how catastrophically the inner future later dies
  (clean `Err`, or the same raw JS throw), there is no guard left open on `App`/`self` to
  get stuck, because none was ever held past the synchronous return. This directly closes
  the exact gap RFC-007 diagnoses, for both failure modes — not just the clean-`Result::Err`
  one the original code already handled correctly. ✅ (AC1, part 2)

On success: `*gpu_cell.borrow_mut() = Some(Arc::new(gpu))` — ordinary `RefCell` interior
mutability, unrelated to wasm-bindgen's own tracking, confirmed nothing here can affect
the JS-visible guard. ✅

## 3. Double-borrow risk — the specific thing the report itself flagged

The report's own "Reviewer notes" ask me to check whether `context()`'s new
`self.gpu.borrow().clone()` could panic via `RefCell`'s conflicting-borrow rule if some
other in-scope code already holds `self.gpu.borrow()` open when `context()` is called.
Independently grepped every `self.gpu` access site in `app.rs` (not just re-read the
report's own claim):

```
line 130: let gpu_cell = self.gpu.clone();        <- Rc::clone, not a RefCell borrow
line 603: gpu: self.gpu.borrow().clone(),          <- the only .borrow() call in the file
```

Exactly one `.borrow()` call exists anywhere in `app.rs`, inside `context()` itself, and
its guard is a temporary dropped at the end of that one statement. `execute_operation`,
`create_node`, `render_tick`, `preview_tick` all call `self.context(...)` but never
`self.gpu.borrow()` directly, and nothing calls `context()` from within a scope that
already holds a `self.gpu` borrow open (there is no such scope anywhere in the file — the
only borrow site *is* inside `context()`). No double-borrow path exists. ✅ (confirms the
report's own claim independently, not just accepting it)

## 4. Unchanged surfaces — AC3, AC4

`Context.gpu`'s own field type (`compositor/context.rs:54`, `pub gpu: Option<Arc<GpuState>>`)
is untouched by this diff — `context()` still hands out the same `Option<Arc<GpuState>>`
value, just sourced via `self.gpu.borrow().clone()` instead of `self.gpu.clone()`. ✅ (AC3)

No operation file, no `gpu/mod.rs`, nothing under `operations/` touched by this diff —
RFC-005/006's regression suites are unaffected by construction (confirmed via `git diff
--stat`, this file isn't in that list). ✅ (AC4, as far as static inspection can confirm
without running them)

## 5. Dependencies and JS call site — not new, unguarded surface

`js_sys` (`0.3.103`) and `wasm-bindgen-futures` (`0.4.76`) are both pre-existing
`Cargo.toml` dependencies already used elsewhere (`wasm_bindgen_futures::spawn_local` in
all 16 GPU-backed operations) — `future_to_promise` isn't a new crate, just a different
function from an already-integrated one. `ui/scripts/app.js`'s call site
(`wasmApp.init_gpu().then(...).catch(...)`) is unaffected — both the old
`#[wasm_bindgen] async fn` and the new `future_to_promise`-based `fn` produce the same
JS-visible `Promise` shape; confirmed no other file references `init_gpu`. ✅

## 6. AC2 — the one criterion neither the developer nor I could satisfy

RFC-007's AC2 asks for a test proving other `App` methods keep working after an
`init_gpu` failure, or — if the harness genuinely can't simulate that — a manual
verification instead. Neither is possible in this sandbox: no wasm32 build target
reachable (network-blocked), no browser to observe `wasm-bindgen`'s runtime borrow-guard
behavior even if a build succeeded. This is **not a defect in the fix** — it's an
environment limitation both the developer and I hit independently — but it is the one
acceptance criterion genuinely unverified beyond source-level reasoning, and given this
bug's critical, total-lockup severity, that reasoning (however carefully traced) is not a
substitute for seeing it actually behave correctly in a browser.

## Decision

**Approve, on the structural/source-level merits — with the browser-verification caveat
carried forward, not waived.** The borrow-lifetime fix is sound by direct trace of Rust's
`async fn` desugaring and wasm-bindgen's documented per-object borrow-guard behavior; the
double-borrow risk the developer flagged for a second look is independently confirmed
absent; scope, dependencies, and unaffected surfaces all check out. Given this is a
critical, total-app-lockup bug and neither this session nor the developer's could verify
the fix against a real wasm32/browser runtime, **recommend Management verify against an
actual build before treating RFC-007 as fully closed**, per the report's own
recommendation — this approval covers the Code Review step of the Implementation Review
Loop, not a substitute for that runtime confirmation.
