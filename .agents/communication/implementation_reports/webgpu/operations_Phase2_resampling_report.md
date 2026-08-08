# Implementation Report: WebGPU operations, Phase 2 — resampling ops

REPORT-ID: REPORT-webgpu-operations-phase2-resampling
Created: 2026-08-08
Created-By: Software Developer
Target-Role: Code Reviewer
Related-Specification: SPECwebgpuoperations.md (Phase 2)
Status: Open

Branch: `claude/agent-setup-prep-oyj2ri`
Files modified (one commit each, independently revertible):

- `engine/src/operations/transform/resize.rs` (`0f7d05b`)
- `engine/src/operations/transform/move_op.rs` (`0634442`)

## Summary

Implements `SPECwebgpuoperations.md`'s Phase 2: GPU-backed dispatch for
the two resampling operations, `RESIZE` and `MOVE`. Both are single-input,
single-buffer shaders (3 bindings: input storage read-only, output
storage read_write, uniform params) - structurally closer to `BLUR`/
Phase 1.1's single-input shape than Phase 1.3's two-buffer one - but
unlike every prior phase's pointwise shaders, each invocation computes a
*transformed* source address (destination -> source, inverse-mapped)
rather than reading its own `(x, y)`. This closes out
`SPECwebgpuoperations.md` in its entirety (Phases 0, 1.1, 1.2, 1.3, 2 all
now implemented).

## Per-operation notes

- **`RESIZE`**: no `MASK` input at all (confirmed via `metadata()`'s own
  comment - a MASK needs matching dimensions against the identity/result
  pair, but RESIZE's output is a different size at any scale != 100%), so
  GPU dispatch is unconditional when available - same shape as
  `RGB_TO_HSV`/`CHECKERBOARD` in that regard. Gated defensively on
  `algorithm == ResizeAlgorithm::NearestNeighbor` (the only variant that
  exists, and the only one the shader implements) so a future `BILINEAR`
  variant falls back to CPU rather than silently computing the wrong
  resample on GPU - same defensive-gate pattern `RGB_TO_HSV` used for
  `format == ColorFormat::Hsv`. `inv_x`/`inv_y` (`100.0 / scale`) are
  precomputed Rust-side in `f64`, exactly matching `resize_pixels`'s own
  formula, then cast to `f32` and uploaded via `bitcast`/`to_bits` - not
  recomputed from raw `SCALE_X`/`SCALE_Y` inside the shader itself, to
  keep the GPU and CPU paths deriving the same intermediate value the
  same way.
- **`MOVE`**: the second operation in this spec not yet migrated to
  bbox-consumption (after `MULTIPLY` in Phase 1.3) - `execute()`'s masked
  path has never called `find_bbox`/`compute_within_bbox` (only
  `output_bbox()` does), so it's always been an unrestricted full-frame
  `move_pixels` + `apply_mask`. I left that path entirely untouched (GPU
  dispatch only applies when `mask.is_none()`), same blanket-rule split
  as `MULTIPLY` and every other operation - consistent with the same
  reasoning the reviewer already confirmed correct for `MULTIPLY`: the
  spec's blanket rule has no "unless already full-frame" exception, and
  keeping bbox-migration and GPU-acceleration as separable workstreams
  avoids conflating the two. Coordinate math is `resize_pixels`'s
  inverse-mapping minus the center-relative scale term (`src = dest -
  offset`, not `src = center + (dest - center) * inv_scale`) - `offset_x`/
  `offset_y` are uploaded the same `bitcast`/`to_bits` way as RESIZE's
  `inv_x`/`inv_y`.

## Architecture notes

- No `Cargo.toml` changes.
- `Resize`'s fingerprint is `{ source, scale_x_bits, scale_y_bits }`
  (both scales via `f64::to_bits()`, same exact-equality pattern every
  prior phase uses for scalar parameters); `Move`'s is `{ source,
  offset_x_bits, offset_y_bits }`, same shape.
- `Resize`'s struct previously had only public `f64`/enum fields
  (constructed directly nowhere outside this file per a full-tree
  `grep`), so no external call sites needed updating. `Move`'s struct,
  however, is directly constructed as a literal (`Move { offset_x, ...
  }`) six times in its own test module (not just via `Move::new()`) -
  all six were updated to `..Move::new()` to pick up the new private
  GPU-state fields, the same mechanical fix `Blur`'s single test literal
  needed in Phase 0. Grepped the whole `engine/src` tree for both
  `Resize {` and `Move {` afterward to confirm no other call sites exist.

## Tests executed

Two new tests per operation (four total), following the established
`is_live_is_true_only_while_a_gpu_dispatch_is_pending` /
`gpu_<op>_matches_cpu_within_tolerance_once_warmed_up` shape. All
pre-existing tests in both files are otherwise unchanged (aside from the
six `Move { ... }` literal fixes noted above, which only add
`..Move::new()` and change no asserted values).

## Test results

**Same restriction as every prior phase - could not run `cargo build`/
`cargo test`.** Re-confirmed immediately before writing this report:
`index.crates.io` still 403. **Acceptance criteria requiring a working
`cargo build`/`cargo test` are UNVERIFIED, not passing.** Hand-traced
against `blur.rs`/`rgb_to_hsv.rs`/`multiply.rs`'s already-approved API
usage and each operation's existing CPU implementation
(`resize_pixels`/`move_pixels`).

## Known limitations

- No GPU adapter or compiler available in this sandbox - nothing here
  has executed even once.
- `MOVE`'s not-yet-migrated masked path is pre-existing, unrelated to
  this change - noted for completeness, not something this phase
  introduced or is expected to fix (same note Phase 1.3's report made
  for `MULTIPLY`).
- This closes out `SPECwebgpuoperations.md` in its entirety once
  reviewed - no further phases remain in the spec.

## Specification deviations

None identified.

## Reviewer notes

Same ask as every prior phase: please treat every claim as needing
independent verification. One thing worth your specific attention, new
to this phase: `RESIZE`'s output-bounds check
(`src_x < 0.0 || src_y < 0.0 || src_x >= f32(width) || src_y >=
f32(height)`) mirrors `resize_pixels`'s own `continue` condition exactly,
but is evaluated in `f32` on GPU vs. `f64` on CPU - same
precision-tolerance story as every prior numerical-tolerance test, but
since this is a *boundary* comparison (not an arithmetic result), a
value sitting exactly on the boundary could in principle round
differently between `f32` and `f64` and pick a different branch (in vs.
out of frame) rather than just a slightly different in-range value. I
didn't find a concrete case where this diverges for either operation's
new test data, but flagging it since it's a different kind of
floating-point risk than anything Phase 0-1.3 needed to consider. If you
find anything not approvable in either operation, please respond with an
RFC naming which one - the two commits are independent.
