REPORT-ID: REPORT-RFC-007-DELIVERED
Created: 2026-08-09
Created-By: software_developer
Target-Role: software_architect
Related-Specification: SPEC-webgpu-compute-backend ("Correction — 2026-08-08" under "App boot") / RFC-007
Status: Delivered

Summary: RFC-007 (critical: App.gpu wasm-bindgen borrow lockup)
implemented, approved by Code Reviewer, and merged to `dev`. Fixes the
bug that permanently bricked the whole app after a single WebGPU init
failure - Management's originally-reported "everything is broken, cannot
load videos" symptom, which RFC-006 did not actually fix since it never
touched this path.

Implemented: See
`.agents/communication/implementation_reports/RFC007_gpu_borrow_lockup_fix_report.md`
for full technical detail. Summary: `App.gpu` changed from a plain
`Option<Arc<GpuState>>` field to `Rc<RefCell<Option<Arc<GpuState>>>>`;
`init_gpu` changed from `async fn(&mut self)` (which held a
`wasm-bindgen` object-borrow of `self` across its `.await`, permanently
stuck if a raw JS exception escaped mid-poll) to a plain
`fn(&self) -> js_sys::Promise` via `future_to_promise`, whose inner
`async move` block captures only a cloned `Rc`, never `self` - so no
failure mode inside the GPU negotiation can ever hold a `self`-borrow
open. JS call site unaffected.

Files modified: `engine/src/app.rs`.

Architecture notes: `future_to_promise` (a plain fn returning
`js_sys::Promise`) was chosen over keeping `async fn` sugar, since Rust's
`async fn` desugaring ties the returned future's lifetime to `self` at
the type level regardless of whether the body still references `self`
late in its execution - the exact mechanism this bug exploited.

Tests executed: None new - `app.rs` is `wasm32`-only and has zero
existing tests.

Test results: **Unverified by both Software Developer and Code
Reviewer**, independently confirmed compounding reasons: sandbox network
policy blocks both ordinary `cargo test` and a `wasm32-unknown-unknown`
target build (`index.crates.io` 403 either way); this repo has no
committed wasm32 build script; and this sandbox has no browser to
execute a compiled module in even if a build succeeded. Both roles
reasoned through the fix's correctness via Rust's `async fn`/borrow-
lifetime semantics and `wasm-bindgen`'s documented object-borrow-guard
behavior rather than executing it.

**Recommendation from both Software Developer and Code Reviewer, given
this RFC's critical severity: verify against a real browser build before
treating RFC-007 as fully closed**, regardless of this Code Review
approval - this is the one acceptance criterion (AC2: other `App`
methods survive an `init_gpu` failure) neither of us could actually
execute, only reason about. RFC-007's own acceptance condition already
names Management approval as the final step for this RFC (it was
Management's own bug report) - flagging this explicitly so that approval
step includes an actual runtime check, not just a read of this report.

Known limitations: Per above - no runtime verification performed by
either role in the Implementation Review Loop for this RFC.

Specification deviations: None from the required behavior.
`future_to_promise` vs. `async fn` sugar is a structural implementation
choice within RFC-007's own stated flexibility, documented in the
earlier report.

Approval-ID: See
`.agents/communication/evaluation/evaluation_rfc007_gpu_borrow_lockup_fix.md`
("Approve") and
`.agents/communication/rfi/RFIresponserfc007gpuborrowlockupreadyformerge.md`.
RFC-ID: RFC-007 (`.agents/communication/rfc/RFC007initgpuborrowlockup.md`).
Merge commit: `ae8259c` on `dev`.
