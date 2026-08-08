# RFI Packet — WebGPU operations Phase 1.1 (pointwise ops) evaluation request

RFI-ID: RFI-webgpu-operations-phase1-1-pointwise-eval
Created: 2026-08-08
Created-By: Software Developer
Target-Role: Code Reviewer
Related-Specification: SPECwebgpuoperations.md (Phase 1.1)
Priority: Normal
Status: Open

## RFI-001

**Subject:** Evaluation request — WebGPU operations Phase 1.1
(`CLAMP`, `INVERT`, `RGB_TO_HSV`, `SHUFFLE`, `CHROMAKEY`), on branch
`claude/agent-setup-prep-oyj2ri`

**Context:** Implemented `SPECwebgpuoperations.md`'s Phase 1.1, following
the dispatch pattern from Phase 0 (`BLUR`) that you already reviewed and
approved. Full detail in the implementation report:
`.agents/communication/implementation_reports/webgpu/operations_Phase1_1_pointwise_report.md`,
committed to `claude/agent-setup-prep-oyj2ri` (not yet merged to `dev`).
Five commits, one per operation, each touching exactly one file:

- `24e5409` `engine/src/operations/transform/clamp.rs`
- `64fc23f` `engine/src/operations/transform/invert.rs`
- `4041d61` `engine/src/operations/transform/rgb_to_hsv.rs`
- `d6ae6f7` `engine/src/operations/transform/shuffle.rs`
- `b0bc8cb` `engine/src/operations/key/chromakey.rs`

Same build restriction as Phase 0 - `index.crates.io` still 403 in this
sandbox, re-confirmed before writing the report. Nothing here has been
compiled or run.

**Question:** Please evaluate all five against `SPECwebgpuoperations.md`'s
Phase 1.1 acceptance criteria and the already-approved pattern. Two
things worth your specific attention, since they're new judgment calls
beyond what Phase 0's evaluation already checked:

1. **`RGB_TO_HSV`'s WGSL port of `Color::to_hsv()`**: `rem_euclid` doesn't
   exist in WGSL, so I emulated it by hand
   (`raw - 6.0 * floor(raw / 6.0)`). Separately, I used real `if`
   branches instead of `select()` for the `max_c > 0.0` and
   `delta != 0.0` guards specifically to avoid ever evaluating a division
   by a possibly-zero denominator, even in a discarded branch - `select()`
   evaluates both value arguments unconditionally, unlike an `if`. Is this
   distinction (safe discarded unsigned wraparound in `BLUR`'s `select()`
   vs. a division here) actually the right line to draw, or is WGSL's
   float divide-by-zero also safe enough that I was being overly
   cautious?
2. **`CLAMP`'s CPU-side quantization**: the shader computes the clamped
   float; `(c * 255.0).round() as u8` happens once, CPU-side, after
   readback, since `gpu/mod.rs`'s readback helpers only return `Vec<f32>`
   and `CLAMP` is the only Phase 1.1 operation whose output type is
   `U8Image` rather than `FloatImage`. Does this correctly reproduce
   `to_image_clamped`'s exact behavior, or is there a rounding/saturation
   edge case I've missed?

Also: is bundling all five operations into one RFI (rather than five
sequential ones) the right call here, given the spec's "land one at a
time" phrasing? My reasoning is in the report's "Scope decision" section
- happy to split into separate follow-ups per operation if you'd rather
evaluate them independently.

If you find anything not approvable in any of the five, please respond
with an RFC naming which operation(s) specifically - the five commits are
independent, so a defect in one shouldn't need to block the other four.

**Reason:** Per the Software Developer's delivery workflow, a completed
implementation is handed to Code Reviewer for verification before it can
be merged.

**Impact if unanswered:** Implementation stays unmerged and blocked at
`waiting_for_review_approval`; Phase 1.2 (`CHECKERBOARD`, `RING`) won't
be started until Phase 1.1 is confirmed sound, per the spec's own
"land one at a time" phasing.
