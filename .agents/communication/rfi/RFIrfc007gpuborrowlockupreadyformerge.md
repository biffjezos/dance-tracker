RFI-ID: RFI-RFC-007-READY
Created: 2026-08-09
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-webgpu-compute-backend ("Correction — 2026-08-08" under "App boot") / RFC-007
Priority: critical
Status: Open

Subject: Is the App.gpu borrow-lockup fix (RFC-007, critical) ready to merge?

Context: RFC-007 (Management-reported, critical: the entire app
permanently stops responding to any interaction after a single WebGPU
init failure, requiring a reload). Root cause: `init_gpu`'s
`async fn(&mut self)` held a `wasm-bindgen` object-borrow of `self`
across its `.await`; a raw JS `TypeError` escaping mid-poll (adapter
request throwing directly rather than resolving to `Result::Err`)
bypassed the normal poll-return path that releases that borrow, leaving
it stuck forever and breaking every subsequent `App` method call
(`execute_operation`, `create_node`, `render_tick`, everything).

Implemented on branch `claude/dev-session-plw4fq`. Full detail in
`.agents/communication/implementation_reports/RFC007_gpu_borrow_lockup_fix_report.md`.

Summary of the change (`engine/src/app.rs` only):
1. `App.gpu`: `Option<Arc<GpuState>>` -> `Rc<RefCell<Option<Arc<GpuState>>>>`.
2. `init_gpu`: `async fn(&mut self) -> Result<(), JsValue>` ->
   `fn(&self) -> js_sys::Promise`, using `wasm_bindgen_futures::future_to_promise`
   with an `async move` block that captures only a cloned `Rc` - never
   `self` - so no `wasm-bindgen` self-borrow can ever be held across the
   `.await`, regardless of how `GpuState::new()` fails.
3. `context()`: `self.gpu.clone()` -> `self.gpu.borrow().clone()`.
4. JS call site (`ui/scripts/app.js`) unaffected - `future_to_promise`
   returns the same JS-visible `Promise` shape `.then()/.catch()` already
   expects.

Question: Does this satisfy RFC-007's required change and acceptance
criteria? Ready to approve for merge to `dev`, given the critical
severity?

Reason / important caveat: I could not verify this at all beyond manual
code-reading and borrow-lifetime reasoning, for two compounding reasons -
both flagged in detail in the implementation report:
1. `cargo build`/`cargo test` blocked by sandbox network policy (same
   `index.crates.io` restriction as RFC-005/006's precedent).
2. `app.rs` is `#![cfg(target_arch = "wasm32")]`-gated and has zero
   existing tests - it's excluded from native `cargo test` entirely even
   with working network access, and this repo has no committed wasm32
   build script. Verifying the actual fix (that `wasm-bindgen`'s runtime
   borrow guard behaves as reasoned) requires a real wasm32 build in a
   browser, which this sandbox cannot provide.

Given RFC-007's critical severity and that this is the one acceptance
criterion (AC2: prove other App methods keep working after an init_gpu
failure) I could not satisfy even manually, I'd value your independent
read on the borrow-lifetime reasoning before this merges, and would
recommend Management verify against a real browser build before
considering RFC-007 fully closed regardless of Code Review outcome here.

Impact if unanswered: RFC-007 stays open past its Implementation Review
Loop step 2; the critical app-breaking bug stays live in `dev`.
