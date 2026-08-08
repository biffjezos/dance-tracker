# RFI Packet — WebGPU operations Phase 0 (BLUR) evaluation request

RFI-ID: RFI-webgpu-operations-phase0-blur-eval
Created: 2026-08-08
Created-By: Software Developer
Target-Role: Code Reviewer
Related-Specification: SPECwebgpuoperations.md (Phase 0)
Priority: Normal
Status: Open

## RFI-001

**Subject:** Evaluation request — WebGPU operations Phase 0 (`BLUR`), on
branch `claude/agent-setup-prep-oyj2ri`

**Context:** Implemented `SPECwebgpuoperations.md`'s Phase 0 (`BLUR`'s
GPU-backed unmasked path), per the Software Architect's handoff recorded
in their working state (`SPECwebgpuoperations.md` v1, `handoff.status:
sent`) and the precondition (`SPECwebgpucomputebackend-1.md` Phase 0)
already merged on `dev`. Full detail in the implementation report:
`.agents/communication/implementation_reports/webgpu/operations_Phase0_blur_report.md`,
committed to `claude/agent-setup-prep-oyj2ri` (not yet merged to `dev`).
Only `engine/src/operations/transform/blur.rs` was modified.

Could not run `cargo build`/`cargo test` at all in this sandbox -
`index.crates.io` is blocked (403), matching the restriction already on
record from your own `notification_cargo_registry_index_blocked.md`.
Every claim in the report is hand-traced, not build-verified - flagged
explicitly so evaluation doesn't assume any of it already passed a
compiler.

**Question:** Please evaluate this implementation against
`SPECwebgpuoperations.md`'s Phase 0 acceptance criteria and
`SPECwebgpucomputebackend-1.md`'s pattern (fingerprint/dispatch/
`is_live()`/numerical-tolerance requirements). In particular:

1. Is the `Rc<RefCell<...>>` sharing between `self` and the wasm32
   `spawn_local` task's captured state correct and sufficient, given
   `execute(&self, ...)` doesn't give the spawned `'static` closure any
   other way to reach back into `self`'s own fields? (See the
   report's "Implemented" section - I flagged this as an ownership
   question the pattern spec doesn't spell out.)
2. Is the WGSL shader (single-pass 2D window average, `vec4<u32>` uniform
   params, bind group layout) correct against real wgpu/WGSL semantics -
   I have no way to cross-check this against real source the way your
   own Phase 0 foundation evaluation did.
3. Does the masked path genuinely stay byte-for-byte unchanged (per the
   blanket rule), and does `is_live()`'s regression test actually cover
   the failure mode the pattern spec requires, given native's blocking
   dispatch means `pending` is never observably `Some` in a real run (see
   the report's "Tests executed" section for how I handled that)?

If you find anything not approvable, please respond with an RFC (not an
RFI response) so the required change is explicit and actionable directly,
rather than needing a follow-up question round first. Otherwise, an RFI
response / evaluation approval is all that's needed.

**Reason:** Per the Software Developer's delivery workflow
(`instructions_software_developer.md`), a completed implementation is
handed to Code Reviewer for verification before it can be merged - this
is the largest and first operation-level WebGPU change in the codebase
(no prior per-operation Phase 0 evaluation exists to pattern-match
against, only the foundation-level one), so getting review before
Phase 1.1 replicates the same pattern nine more times matters more than
usual here.

**Impact if unanswered:** Implementation stays unmerged and blocked at
`waiting_for_review_approval` in my working state; Phase 1.1 onward
(the other nine operations) won't be started until this gating phase is
confirmed sound, per the spec's own acceptance criterion #1.
