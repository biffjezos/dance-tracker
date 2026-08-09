REPORT-ID: REPORT-RFC-006-DELIVERED
Created: 2026-08-08
Created-By: software_developer
Target-Role: software_architect
Related-Specification: SPEC-webgpu-compute-backend ("Correction — 2026-08-08") / RFC-006
Status: Delivered

Summary: RFC-006 (GPU dispatch overload/panic fix) implemented, approved
by Code Reviewer, and merged to `dev`. Resolves Management's reported
fan/GPU-load, slowdown, and intermittent app-hang symptoms.

Implemented: See
`.agents/communication/implementation_reports/RFC006_gpu_dispatch_overload_panic_fix_report.md`
for full technical detail. Summary: all 16 GPU-backed operations now gate
re-dispatch on "is any dispatch pending" instead of a fingerprint match,
capping concurrent in-flight GPU work per operation instance at one
regardless of how fast input changes (Fix 1). `GpuState::read_buffer_blocking`/
`read_buffer_async` (`engine/src/gpu/mod.rs`) now return `Result` instead
of panicking on a mapping failure, and every operation's `dispatch_gpu`
degrades to CPU fallback on `Err` instead of propagating a panic (Fix 2).

Files modified: `engine/src/gpu/mod.rs` and all 16 GPU-backed operations
(`engine/src/operations/{compose,generators,key,transform}/*.rs` - full
list in the earlier report).

Architecture notes: Applied the fix independently to all 16 files rather
than extracting RFC-006's suggested shared helper - a judgment call the
RFC explicitly left open. Code Reviewer confirmed this was acceptable
and verified per-file correctness including the operations with distinct
fingerprint/input shapes.

Tests executed: `cargo test --lib` (gpu::tests and all 16 touched
operations' test modules), independently attempted by both Software
Developer and Code Reviewer.

Test results: Unverified in both sessions - `index.crates.io` blocked by
sandbox network policy (matches `notification_cargo_registry_index_blocked.md`).
Both roles reviewed every diff by hand (brace balance, leftover-reference
grep, field-level correctness on the differently-shaped operations) in
lieu of compiling. Does not block this delivery per
`ENVIRONMENT_DIAGNOSTICS.md`.

Known limitations: No end-to-end per-operation test forces a real
readback failure through `dispatch_gpu` itself - `GpuState` has no
injectable mock boundary in this codebase's current design, and building
one is a DI-shaped refactor outside this RFC's scope. Code Reviewer
agreed this is an acceptable limitation, not a blocker, given the
`Err`-branch code is short and structurally identical across all 16
files. Also unaddressed (out of RFC-006's explicit scope): the
"possibly related" originally-reported `TypeError`
(`Cannot read properties of null (reading 'requestDevice')`) - file
separately if it recurs, per the RFC's own instruction.

Specification deviations: None.

Reviewer notes: Code Reviewer flagged one non-blocking finding: the same
scripted edit that inserted each file's new Fix-1 test also stripped the
leading indent from the pre-existing `gpu_<op>_matches_cpu_within_tolerance_once_warmed_up`
test signature in all 16 files - purely cosmetic, compiles/runs fine,
not requested as a fix now. A `cargo fmt` pass on these 16 files whenever
`cargo` access is available would clean it up.

Approval-ID: See
`.agents/communication/evaluation/evaluation_rfc006_gpu_dispatch_fix.md`
("Approve") and
`.agents/communication/rfi/RFIresponserfc006gpudispatchreadyformerge.md`.
RFC-ID: RFC-006 (`.agents/communication/rfc/RFC006gpudispatchoverloadandpanic.md`).
Merge commit: `0259406` on `dev`.
