# RFI Packet — WebGPU operations Phase 2 (resampling ops) evaluation request

RFI-ID: RFI-webgpu-operations-phase2-resampling-eval
Created: 2026-08-08
Created-By: Software Developer
Target-Role: Code Reviewer
Related-Specification: SPECwebgpuoperations.md (Phase 2)
Priority: Normal
Status: Open

## RFI-001

**Subject:** Evaluation request — WebGPU operations Phase 2 (`RESIZE`,
`MOVE`), on branch `claude/agent-setup-prep-oyj2ri`

**Context:** Implemented `SPECwebgpuoperations.md`'s Phase 2, following
the pattern from Phases 0/1.1/1.2/1.3. Full detail in the implementation
report:
`.agents/communication/implementation_reports/webgpu/operations_Phase2_resampling_report.md`,
committed to `claude/agent-setup-prep-oyj2ri` (not yet merged to `dev`).
Two commits, one per operation:

- `0f7d05b` `engine/src/operations/transform/resize.rs`
- `0634442` `engine/src/operations/transform/move_op.rs`

Same build restriction as every prior phase - `index.crates.io` still
403, re-confirmed before writing the report.

**Question:** Please evaluate both against Phase 2's acceptance criteria.
Two things specific to this phase, worth your attention:

1. **`MOVE`'s masked-path scope:** like `MULTIPLY` in Phase 1.3, `MOVE`
   was never migrated to bbox-consumption - `execute()`'s masked path has
   never called `find_bbox`/`compute_within_bbox` (only `output_bbox()`
   does), so it's always been an unrestricted full-frame `move_pixels` +
   `apply_mask`. I left it entirely untouched (GPU dispatch only applies
   when `mask.is_none()`), applying the same reasoning you already
   confirmed correct for `MULTIPLY`. Does that reasoning still hold here,
   or is there something about `MOVE` specifically that should change the
   call?
2. **`RESIZE`'s `f32` vs. `f64` boundary check:** `resize_pixels`'s
   out-of-frame `continue` condition (`src_x < 0.0 || ... || src_x >=
   width`) is ported to the GPU shader verbatim, but evaluated in `f32`
   there vs. `f64` on CPU. Every prior numerical-tolerance test only
   needed to worry about `f32`/`f64` producing slightly different
   in-range *values*; this is a boundary comparison, so in principle a
   value sitting exactly on the edge could evaluate to a different
   branch (transparent vs. sampled) between the two precisions, not just
   a slightly different number. I didn't find a concrete failing case for
   either operation's own test data, but wanted to flag the category of
   risk explicitly since it's new to this phase. Worth a specific look,
   or is this the same already-accepted precision-tolerance story as
   every prior phase?

If you find anything not approvable in either operation, please respond
with an RFC naming which one - the two commits are independent, same as
every prior phase.

**Reason:** Per the Software Developer's delivery workflow, a completed
implementation is handed to Code Reviewer for verification before it can
be merged. This closes out `SPECwebgpuoperations.md` in its entirety
(Phases 0, 1.1, 1.2, 1.3, 2) once reviewed.

**Impact if unanswered:** Implementation stays unmerged; the spec remains
incomplete.
