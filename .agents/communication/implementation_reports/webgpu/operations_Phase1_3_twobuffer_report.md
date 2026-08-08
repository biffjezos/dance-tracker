# Implementation Report: WebGPU operations, Phase 1.3 — pointwise two-buffer ops

REPORT-ID: REPORT-webgpu-operations-phase1-3-twobuffer
Created: 2026-08-08
Created-By: Software Developer
Target-Role: Code Reviewer
Related-Specification: SPECwebgpuoperations.md (Phase 1.3)
Status: Open

Branch: `claude/agent-setup-prep-oyj2ri`
Files modified (one commit each, independently revertible):

- `engine/src/operations/compose/add.rs` (`1833eb2`)
- `engine/src/operations/compose/screen.rs` (`e77d42a`)
- `engine/src/operations/compose/subtract.rs` (`12ea60a`)
- `engine/src/operations/compose/multiply.rs` (`be15241`)
- `engine/src/operations/compose/mix.rs` (`828f5c0`)
- `engine/src/operations/key/hue_key.rs` (`b6ac44a`)

## Summary

Implements `SPECwebgpuoperations.md`'s Phase 1.3: GPU-backed dispatch for
all six two-buffer pointwise operations. Same bind-group shape as
Phase 1.1 but with a second read-only input storage buffer (`Foreground`/
`Background`, or `Source`/`Reference` for `HUE_KEY`) — 4 bindings total
for every operation in this phase (two inputs, one output, one uniform
params buffer). Per-operation dispatch structure is a direct copy of
`Blur`/`Add`'s shape into each file, per the established design
principle.

## Per-operation notes

- **`ADD`, `SCREEN`, `SUBTRACT`**: identical shape, only the per-channel
  formula differs (`a+b`, `1-(1-a)*(1-b)`, `a-b`). Masked path (union of
  Foreground's and Background's boxes, per the bbox work's own
  established reasoning for these three) is untouched.
- **`MULTIPLY`**: the one operation in this phase not yet migrated to
  bbox-consumption — its masked path was already, and remains, an
  unrestricted full-frame CPU compute + `apply_mask`, not a
  work_area-restricted one. GPU dispatch still only applies when
  `mask.is_none()`, same blanket-rule split every other operation uses -
  I did not treat "the masked path already happens to be full-frame" as
  license to accelerate it too, to stay consistent with the established
  pattern rather than introduce a one-off exception.
- **`MIX`**: no `MASK` input at all (confirmed via `metadata()`), so GPU
  dispatch is unconditional - same shape as `RGB_TO_HSV`/`CHECKERBOARD`
  in that regard. One extra uniform value (`AMOUNT`) beyond width/height,
  bit-packed the same way `BLUR`'s `MIN`/`MAX` are.
- **`HUE_KEY`**: two inputs are `SOURCE`/`REFERENCE`, not `Foreground`/
  `Background` - the shader and bindings are named accordingly.
  `target_hue` is computed once, CPU-side, via the already-tested
  `Color::to_hsv()` and passed down as a single uniform float, rather
  than re-porting the full RGB→HSV conversion into WGSL a second time -
  `HUE_KEY` only ever needs the hue scalar, not the full HSV triple.
  `hue_distance` itself *is* ported to WGSL (it runs per-pixel against
  `REFERENCE`'s buffer): its `%` usage doesn't need `RGB_TO_HSV`'s
  floor-mod emulation from Phase 1.1, since the dividend (`abs(a - b)`)
  is always non-negative by construction here, so WGSL's truncated `%`
  and a euclidean one agree exactly - flagged explicitly since it's a
  deliberate departure from the Phase 1.1 precedent, not an oversight.

## Architecture notes

- No `Cargo.toml` changes.
- Every operation resolves both input `FloatImage`s unconditionally
  before branching on `mask` (needed for the pre-existing dimension-match
  check regardless), so - unlike `BLUR`/Phase 1.1's single-input
  operations - there's no double-resolve-via-`FloatImage::from_value` in
  the unmasked GPU-dispatch branch here; `dispatch_gpu` receives clones
  of the already-resolved images instead.

## Tests executed

Two new tests per operation (twelve total), following the established
shapes. All six operations' full pre-existing test suites are unchanged -
no struct-literal fixes needed (checked via `grep` before editing, as
with every prior phase).

## Test results

**Same restriction as every prior phase - could not run `cargo build`/
`cargo test`.** Re-confirmed immediately before writing this report:
`index.crates.io` still 403. **Acceptance criteria requiring a working
`cargo build`/`cargo test` are UNVERIFIED, not passing.** Hand-traced
against `blur.rs`/Phase 1.1/1.2's already-approved API usage and each
operation's existing CPU implementation.

## Known limitations

- No GPU adapter or compiler available in this sandbox - nothing here
  has executed even once.
- `HUE_KEY`'s `%`-safety reasoning (non-negative dividend, no floor-mod
  needed) is a new judgment call extending Phase 1.1's `RGB_TO_HSV`
  precedent to a different operation - worth specific reviewer
  attention, same as every phase's genuinely new reasoning has been.
- `MULTIPLY`'s not-yet-migrated masked path is pre-existing, unrelated
  to this change - noted for completeness, not something this phase
  introduced or is expected to fix.
- Phase 2 (`RESIZE`, `MOVE`) is not implemented - this closes out
  Phase 1 entirely (1.1, 1.2, 1.3 all landed) once reviewed.

## Specification deviations

None identified.

## Reviewer notes

Same ask as every prior phase: please treat every claim as needing
independent verification. Two things worth your specific attention,
both new to this phase: (1) `MULTIPLY`'s masked-path scope decision
(leaving an already-unrestricted CPU path untouched rather than
extending GPU dispatch to it) - is that the right call, or should its
masked path have been accelerated too since it's already full-frame?
(2) `HUE_KEY`'s `%` safety argument for skipping the floor-mod emulation
`RGB_TO_HSV` needed. If you find anything not approvable in any of the
six, please respond with an RFC naming which operation(s) - the six
commits are independent.
