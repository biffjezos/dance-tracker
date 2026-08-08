# RFI Packet — WebGPU operations Phase 1.3 (two-buffer ops) evaluation request

RFI-ID: RFI-webgpu-operations-phase1-3-twobuffer-eval
Created: 2026-08-08
Created-By: Software Developer
Target-Role: Code Reviewer
Related-Specification: SPECwebgpuoperations.md (Phase 1.3)
Priority: Normal
Status: Open

## RFI-001

**Subject:** Evaluation request — WebGPU operations Phase 1.3
(`ADD`, `SCREEN`, `SUBTRACT`, `MULTIPLY`, `MIX`, `HUE_KEY`), on branch
`claude/agent-setup-prep-oyj2ri`

**Context:** Implemented `SPECwebgpuoperations.md`'s Phase 1.3, following
the pattern from Phases 0/1.1/1.2. Full detail in the implementation
report:
`.agents/communication/implementation_reports/webgpu/operations_Phase1_3_twobuffer_report.md`,
committed to `claude/agent-setup-prep-oyj2ri` (not yet merged to `dev`).
Six commits, one per operation:

- `1833eb2` `engine/src/operations/compose/add.rs`
- `e77d42a` `engine/src/operations/compose/screen.rs`
- `12ea60a` `engine/src/operations/compose/subtract.rs`
- `be15241` `engine/src/operations/compose/multiply.rs`
- `828f5c0` `engine/src/operations/compose/mix.rs`
- `b6ac44a` `engine/src/operations/key/hue_key.rs`

Same build restriction as every prior phase - `index.crates.io` still
403, re-confirmed before writing the report.

**Question:** Please evaluate all six against Phase 1.3's acceptance
criteria. Two things specific to this phase, worth your attention:

1. **`MULTIPLY`'s masked-path scope:** unlike `ADD`/`SCREEN`/`SUBTRACT`,
   `MULTIPLY` was never migrated to bbox-consumption - its masked path is
   still an unrestricted full-frame CPU compute + `apply_mask`. I left it
   entirely untouched (GPU dispatch only applies when `mask.is_none()`,
   same as every other operation), rather than treating "it's already
   full-frame" as license to accelerate that path too. Is that the
   correct, conservative call, or should `MULTIPLY`'s masked path have
   gotten GPU dispatch as well since there's no bbox-restriction logic to
   preserve there in the first place?
2. **`HUE_KEY`'s `%` safety argument:** `RGB_TO_HSV` (Phase 1.1) needed a
   hand-rolled floor-mod emulation for its hue wraparound because its
   input could be negative. `HUE_KEY`'s `hue_distance` also uses `%`, but
   I reasoned its dividend (`abs(a - b)`) is always non-negative by
   construction, so WGSL's truncated `%` and a euclidean one agree
   exactly here, and I used plain `%` without the emulation. Is that
   reasoning sound?

If you find anything not approvable in any of the six, please respond
with an RFC naming which operation(s) - the six commits are independent,
same as every prior phase.

**Reason:** Per the Software Developer's delivery workflow, a completed
implementation is handed to Code Reviewer for verification before it can
be merged. This closes out Phase 1 entirely (1.1, 1.2, 1.3) once
reviewed.

**Impact if unanswered:** Implementation stays unmerged; Phase 2
(`RESIZE`, `MOVE`) won't be started until Phase 1.3 is confirmed sound.
