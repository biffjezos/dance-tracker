RFI-ID: RFI-RFC-006-READY
Created: 2026-08-08
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-webgpu-compute-backend ("Correction — 2026-08-08") / RFC-006
Priority: high
Status: Open

Subject: Is the GPU dispatch overload/panic fix (RFC-006) ready to merge?

Context: RFC-006 (Management-reported: fan/GPU load, slowdown, and
intermittent app hangs since the WebGPU backend shipped) identified two
defects in the pipelined GPU dispatch pattern shared by all 16 GPU-backed
operations: (1) the re-dispatch gate checked a fingerprint match instead
of "any dispatch pending," so continuously-changing input (any live
video, any time-varying generator) launched an unbounded number of
concurrent GPU dispatches; (2) `GpuState`'s buffer-readback functions
`.expect()`-ed on a mapping failure, which on `wasm32` runs inside a
detached `spawn_local` task where a panic traps the whole WASM instance
with no recovery path.

Implemented on branch `claude/dev-session-plw4fq`. Full detail in
`.agents/communication/implementation_reports/RFC006_gpu_dispatch_overload_panic_fix_report.md`.

Summary of the change:
1. All 16 operations: re-dispatch gate changed from
   `already_pending = pending.borrow().as_ref().is_some_and(|p| p.matches(&fingerprint))`
   to `has_pending = self.pending.borrow().is_some()`.
2. `engine/src/gpu/mod.rs`: `read_buffer_blocking`/`read_buffer_async` now
   return `Result<Vec<f32>, String>` instead of panicking on a mapping
   failure. All 16 operations' `dispatch_gpu` match on that `Result` in
   both target branches, clearing `pending` and falling back to CPU on
   `Err` instead of propagating a panic.
3. Structural choice: applied the fix independently to all 16 files
   rather than extracting RFC-006's suggested shared
   `PipelinedDispatch`-shaped helper - a judgment call the RFC explicitly
   leaves open, reasoning given in the implementation report.
4. New tests: one per operation (16 total) proving Fix 1's single-pending
   guarantee, plus one in `gpu/mod.rs` proving Fix 2's `Err`-not-panic
   contract on a deliberately unmappable buffer.

Question: Does this satisfy RFC-006's required changes and acceptance
criteria? Ready to approve for merge to `dev`?

Reason: `cargo build`/`cargo test` are unverified in my session -
`index.crates.io` still blocked (403, "Host not in allowlist"), same
restriction as RFC-005's precedent. Also flagging one known limitation
myself (see the implementation report's "Known limitations"): no test
forces a real readback failure through an operation's own `dispatch_gpu`
end-to-end, since `GpuState` has no injectable mock boundary in this
codebase's current design - the `gpu/mod.rs`-level test covers Fix 2's
contract at the layer it actually lives, and the 16 operations' `Err`-branch
code was verified by direct reading rather than end-to-end. If your
session has working `cargo` access, please run `cargo test --lib` (at
minimum the `gpu::tests` module and the 16 touched operations' test
modules) to confirm, and weigh in on whether the missing end-to-end
failure-injection coverage needs addressing before merge or is an
acceptable gap given the constraint that produced it.

Impact if unanswered: RFC-006 stays open past its Implementation Review
Loop step 2; Management's original stability/performance report stays
unresolved, and the fan/hang symptoms remain live in `dev`.
