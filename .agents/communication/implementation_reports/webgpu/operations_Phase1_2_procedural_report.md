# Implementation Report: WebGPU operations, Phase 1.2 — procedural zero-buffer ops

REPORT-ID: REPORT-webgpu-operations-phase1-2-procedural
Created: 2026-08-08
Created-By: Software Developer
Target-Role: Code Reviewer
Related-Specification: SPECwebgpuoperations.md (Phase 1.2)
Status: Open

Branch: `claude/agent-setup-prep-oyj2ri`
Files modified (one commit each, independently revertible):

- `engine/src/operations/generators/checkerboard.rs` (`7511297`)
- `engine/src/operations/generators/ring.rs` (`55af09e`)

## Summary

Implements `SPECwebgpuoperations.md`'s Phase 1.2: GPU-backed dispatch for
`CHECKERBOARD` and `RING`, both of which have no wired inputs at all (no
`SOURCE`, no `MASK`) - "zero buffer" per the phase's own framing. Same
overall dispatch shape as Phases 0/1.1 (`<Op>GpuPipeline`/
`<Op>Fingerprint`/`Completed<Op>Job`, `Rc<RefCell<...>>`-shared state,
target-conditional readback, `is_live()` override), with two structural
differences forced by having no input:

1. **No `Value` in the fingerprint.** Every previous operation's
   fingerprint included the wired `SOURCE` `Value`, compared via
   `value_ptr_eq`. `CHECKERBOARD`/`RING` have nothing wired, so their
   fingerprints are keyed directly on `ctx.meta.width`/`height` plus
   their own parameters (`size`/`color_a`/`color_b` for `CHECKERBOARD`;
   `count`/`radius`/`spacing`/`thickness`/`colors` for `RING`), compared
   by real equality rather than pointer identity.
2. **No input storage buffer** - the bind group only has an output buffer
   plus params (`CHECKERBOARD`: 2 bindings total) or output + a colors
   buffer + params (`RING`: 3 bindings).

## `RING`'s specific challenge: an unbounded per-ring color list

`RING`'s `colors: Vec<Color>` has no upper bound (`COUNT`'s own parameter
descriptor is `max: None`), so it can't be packed into a fixed-size
uniform buffer the way `CHECKERBOARD`'s two named colors can. Used a
runtime-sized storage buffer instead (`array<vec4<f32>>` at binding 1,
uploaded via the same `gpu.upload()` helper as everything else) - this is
the same WGSL mechanism `gpu/mod.rs`'s own pre-existing test already
relies on for pixel data (`id.x < arrayLength(&input)` in
`DOUBLE_SHADER`), just applied here to a per-ring color table instead.
The shader doesn't call `arrayLength()` itself (the ring-count loop bound
comes from the uniform `count` value, which the Rust side already knows
and keeps in sync with `colors`'s actual length via `Ring::set_count`),
but the underlying buffer-sizing mechanism is the same one that function
demonstrates works.

## Per-operation notes

- **`CHECKERBOARD`**: shader ports the checker math (`((x/tile)+(y/tile))
  % 2`) and `select()`s between the two already-computed color vectors -
  safe, no discarded-branch computation risk (unlike the RGB_TO_HSV case
  RFC-003 was about). `tile = self.size.max(1.0) as u32` is computed
  Rust-side before upload, matching the CPU path's own guard exactly, so
  the shader's `id.x / tile` never divides by zero.
- **`RING`**: shader loop (`for ring_number in 1..=count`) ports
  `generate()`'s own loop directly - same "first ring that matches wins,
  skip negative `ring_radius`, default fully transparent" structure.
  `radius`/`spacing`/`thickness` are `f64` on the Rust struct, packed via
  the same `f64::to_bits()` → `f32` cast → `f32::to_bits()` chain as
  `ChromaKey`'s `THRESHOLD`.
- Both operations' final output is `U8Image`, not `FloatImage` - same
  situation as `CLAMP`. Quantization after readback uses
  `(c.clamp(0.0, 1.0) * 255.0) as u8` - `Color::to_rgba_u8`'s own
  **truncating** cast, deliberately *not* `.round()` like `CLAMP`'s
  `to_image_clamped` uses. Getting this wrong (using the wrong rounding
  rule) would have been an easy, silent, off-by-one-in-the-low-bit kind
  of mistake - flagging explicitly since it's a new distinction this
  phase introduces that didn't exist in Phases 0/1.1 (which only ever
  used one quantization rule, `CLAMP`'s).

## Architecture notes

- No `Cargo.toml` changes.
- `RING`'s colors buffer is uploaded fresh on every dispatch (no
  caching of the buffer itself across dispatches) - consistent with
  every other operation's input buffer here, none of which persist
  buffers across ticks either; only the final `Vec<u8>` result is cached
  in `last_gpu_result`.

## Tests executed

Two new tests per operation, following the established shape:

- `is_live_is_true_only_while_a_gpu_dispatch_is_pending`
- `gpu_<op>_matches_cpu_within_tolerance_once_warmed_up` - `RING`'s uses
  `COUNT=2` with two distinct ring colors specifically to exercise the
  colors-storage-buffer path with more than one entry, not just the
  trivially-correct single-ring default.

Both operations' full pre-existing test suites are unchanged - no
struct-literal fixes needed (checked via `grep` before editing, as with
every prior phase).

## Test results

**Same restriction as every prior phase - could not run `cargo build`/
`cargo test`.** Re-confirmed immediately before writing this report:
`index.crates.io` still 403. Unchanged from Management's open
notification - not re-filing. **Acceptance criteria requiring a working
`cargo build`/`cargo test` are UNVERIFIED, not passing.** Hand-traced
against `blur.rs`/Phase 1.1's already-approved API usage,
`gpu/mod.rs`'s own `arrayLength()` precedent, and each operation's
existing CPU implementation.

## Known limitations

- No GPU adapter or compiler available in this sandbox - nothing here
  has executed even once.
- The `Color::to_rgba_u8` vs. `to_image_clamped` quantization-rule
  distinction (truncating vs. rounding) is a new judgment call this
  phase introduces - flagged above, worth specific reviewer attention.
- Phase 1.3 (`ADD`, `SCREEN`, `SUBTRACT`, `MULTIPLY`, `MIX`, `HUE_KEY`)
  and beyond are not implemented.

## Specification deviations

None identified. The `RING` colors-storage-buffer design isn't spelled
out in the spec (which only says "check `operations/generators/ring.rs`
directly for the exact current shape before writing the shader") - this
is the resolved shape, not a deviation from anything stated.

## Reviewer notes

Same ask as every prior phase: please treat every claim as needing
independent verification, not confirmation of something already known
to work. Two things worth your specific attention, both new to this
phase: (1) the truncating- vs. rounding-cast distinction for
quantization - did I apply it to the right operation in the right
direction; (2) whether the `RING` colors storage buffer's binding/layout
(`read_only: true`, no `min_binding_size`) is correctly shaped for a
runtime-sized array binding specifically, as opposed to the fixed-size
bindings every prior phase used. If you find anything not approvable in
either operation, an RFC naming which one is fine - the two commits are
independent.
