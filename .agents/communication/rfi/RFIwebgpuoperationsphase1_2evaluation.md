# RFI Packet — WebGPU operations Phase 1.2 (procedural ops) evaluation request

RFI-ID: RFI-webgpu-operations-phase1-2-procedural-eval
Created: 2026-08-08
Created-By: Software Developer
Target-Role: Code Reviewer
Related-Specification: SPECwebgpuoperations.md (Phase 1.2)
Priority: Normal
Status: Open

## RFI-001

**Subject:** Evaluation request — WebGPU operations Phase 1.2
(`CHECKERBOARD`, `RING`), on branch `claude/agent-setup-prep-oyj2ri`

**Context:** Implemented `SPECwebgpuoperations.md`'s Phase 1.2, following
the same dispatch pattern from Phases 0/1.1. Full detail in the
implementation report:
`.agents/communication/implementation_reports/webgpu/operations_Phase1_2_procedural_report.md`,
committed to `claude/agent-setup-prep-oyj2ri` (not yet merged to `dev`).
Two commits, one per operation:

- `7511297` `engine/src/operations/generators/checkerboard.rs`
- `55af09e` `engine/src/operations/generators/ring.rs`

Same build restriction as every prior phase - `index.crates.io` still
403, re-confirmed before writing the report.

**Question:** Please evaluate both against Phase 1.2's acceptance
criteria. Two things specific to this phase, worth your attention:

1. **Quantization rule per operation:** both `CHECKERBOARD` and `RING`
   output `U8Image`, same situation as `CLAMP` - but their colors go
   through `Color::to_rgba_u8` (a **truncating** cast, `(c.clamp(0.0,
   1.0) * 255.0) as u8`), not `to_image_clamped` (a **rounding** one,
   `.round() as u8`). I used the truncating rule for both. Did I get the
   right rule for the right operation?
2. **`RING`'s colors storage buffer:** its per-ring color list has no
   upper bound (`COUNT`'s own `max: None`), so it can't fit a fixed-size
   uniform buffer like `CHECKERBOARD`'s two colors. Used a runtime-sized
   storage buffer instead (`array<vec4<f32>>`), the same mechanism
   `gpu/mod.rs`'s own pre-existing test already demonstrates
   (`arrayLength()`), just applied to per-ring color data instead of
   pixel data. Is the bind group layout entry for this binding
   (`BufferBindingType::Storage { read_only: true }`, `min_binding_size:
   None`) correctly shaped for a runtime-sized array specifically?

Also note: both fingerprints are structurally different from every prior
operation's - no wired `Value` exists to compare via `value_ptr_eq`
(these operations have no inputs at all), so they're keyed on
`ctx.meta.width`/`height` plus the operation's own parameters, compared
by real equality instead of pointer identity. Worth confirming this is
the right shape for a no-input operation, not just accepting it by
analogy to the prior phases.

If you find anything not approvable in either operation, please respond
with an RFC naming which one - the two commits are independent, same as
every prior phase.

**Reason:** Per the Software Developer's delivery workflow, a completed
implementation is handed to Code Reviewer for verification before it can
be merged.

**Impact if unanswered:** Implementation stays unmerged; Phase 1.3
(`ADD`, `SCREEN`, `SUBTRACT`, `MULTIPLY`, `MIX`, `HUE_KEY`) won't be
started until Phase 1.2 is confirmed sound.
