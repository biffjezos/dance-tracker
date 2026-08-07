<!-- Handoff snapshot, version 1, delivered 2026-08-07. Canonical/owned copy:
     .agents/roles/software_architect/docs/specifications/SPECwebgpuoperations.md
     (Software Architect). Read that file for future updates - this copy is
     the point-in-time delivery instance for this handoff, not a second
     source of truth to maintain independently. -->

# Specification: GPU-accelerating individual operations

**Precondition:** `SPEC-webgpu-compute-backend.md`'s Phase 0 has landed (the `gpu` module, `Context.gpu`/`App.gpu`, the `App::context()` wiring, and "the pattern every GPU-backed operation follows" section — this spec's every phase implements that pattern, not a redescription of it). Verify with a working `cargo build` and that `Context.gpu: Option<Arc<GpuState>>` compiles before starting Phase 0 below.

**Blanket rule for every phase in this document, stated once:** GPU dispatch only ever replaces an operation's **unmasked** compute path. Every operation below that already has bbox-restricted masked-path logic (`compute_within_bbox`, from the bounding-box work) keeps that path completely untouched — it's already correct and already tested. GPU acceleration is additive to the *unmasked, full-frame* path only, same as `BLUR`'s original design. Don't re-derive this per operation; it applies uniformly.

## Taxonomy — how to classify an operation for GPU work

This is the reusable part, meant to outlive the specific operation list below. When a new operation is added (`CROP` was named as a likely future example) or an existing one needs re-examining, run it through these questions in order:

1. **Does it primarily manipulate pixels at all?** If no — it manages graph wiring/parameters (`PATCH`), or it produces a non-pixel `Value` (`SINE`/`SQUARE`/`LISSAJOUS` produce `Number`), or it's a pixel *source* rather than a pixel *transform* (`CAMERA`/`IMAGE`/`VIDEO` deliver frames, they don't compute new ones from existing ones) — **not applicable to this GPU effort.** Stop here.
2. **Does computing one output pixel require reading pixels other than the corresponding input pixel(s)?**
   - No, and it reads 1-2 whole input buffers, same address as the output → **pointwise/elementwise.** The easiest, most GPU-friendly shape: one shader invocation per pixel, zero cross-invocation dependency.
   - No, but the *address* read is transformed (not the same `(x,y)` as the output) → **resampling.** Nearly as simple as pointwise (still one read per invocation, still no cross-invocation dependency), but the shader computes a source address rather than reading its own.
   - No, and there's no input buffer at all (output is pure function of `(x,y)` and the operation's own parameters) → **procedural pointwise**, a degenerate/simpler case of pointwise (no buffer upload needed at all).
   - Yes → **stencil.** Needs to read a neighborhood, genuinely more shader complexity (workgroup-local memory is usually worth it), and its own reported bounding box typically *grows* rather than staying put (see the bbox work's own Phase 2 for `BLUR`, the only current example).
3. **Does it carry state across ticks** (a history buffer, anything a `RefCell` persists between `execute()` calls independent of this tick's inputs)? If yes — **stateful/temporal**, and not a good near-term GPU candidate regardless of its pixel-shape answer above: the async pipelined-dispatch pattern already introduces one kind of cross-tick state; layering a *second*, operation-owned kind of cross-tick state (like `GHOST`'s history) on top is real, separate design work, not covered here.

**Worked example, since none exists yet:** a future `CROP` (crop to a rectangle, transparent outside it) would answer: manipulates pixels (yes) → reads only the corresponding input pixel, same address, just conditionally blanked outside the rectangle (no neighbor reads) → no state → **pointwise** (specifically, single-buffer, closer to the family in Phase 1.1 below than a new category). It would not need its own phase; it would join whichever pointwise phase is open when it's built.

## Current classification of every operation in the tree (~24 total)

| Category | Operations |
|---|---|
| Not applicable | `CAMERA`, `IMAGE`, `VIDEO` (sources — deliver, don't compute) · `SINE`, `SQUARE`, `LISSAJOUS` (produce `Number`, not pixels) · `PATCH` (parameter injection, not pixel math) |
| Stencil | `BLUR` (only one today) |
| Pointwise — single buffer | `CLAMP`, `INVERT`, `RGB_TO_HSV`, `SHUFFLE`, `CHROMAKEY` |
| Pointwise — zero buffer (procedural) | `CHECKERBOARD`, `RING` |
| Pointwise — two buffer | `ADD`, `SCREEN`, `SUBTRACT`, `MULTIPLY`, `MIX`, `HUE_KEY` |
| Resampling | `RESIZE`, `MOVE` |
| Stateful/temporal — not recommended yet | `GHOST` |

## Phase 0 — `BLUR` (stencil)

The one operation with existing (broken, being replaced) GPU code, and the structurally hardest of the current set — both reasons land on the same operation, independently of each other (see the conversation record if this needs re-explaining to anyone new). Apply "the pattern every GPU-backed operation follows" from `SPEC-webgpu-compute-backend.md` directly: `Blur` gets its own `RefCell`-held `gpu_pipeline`/`pending`/`last_gpu_result`, its own fingerprint `(radius_px, source_value)` compared via `value_ptr_eq`, its own `is_live()` fix, target-conditional dispatch, numerical-tolerance tests. `BlurGpuPipeline`/`PendingBlurJob`/`CompletedBlurJob` are defined in `operations/transform/blur.rs`, not in the `gpu` module.

**What's specifically hard here, beyond the generic pattern:** the WGSL shader itself needs to read a `(2 * radius + 1)`-wide window around each output pixel, not just its own — workgroup-shared-memory tiling is the standard way to avoid every invocation independently re-reading overlapping neighbor data from global memory, but a naive per-invocation neighbor read (correct, just not maximally fast) is an acceptable first cut; don't block Phase 0 of this document on a fully-optimized tiled kernel. `BLUR`'s masked path already grows its own bbox by `radius_px` (bbox work Phase 2) — the GPU shader's dispatch region should match that same grown-and-clamped box when a `MASK` isn't wired (full frame) vs. is (not this phase — see the blanket rule above).

**Acceptance:** the four criteria already specified for this in `SPEC-webgpu-compute-backend.md`'s prior draft — GPU output matches CPU within tolerance once warmed up; identical CPU-only behavior with no GPU; the static-graph regression test for the `is_live()`/stuck-on-CPU bug; full `cargo test` passing natively.

## Phase 1.1 — Pointwise, single buffer: `CLAMP`, `INVERT`, `RGB_TO_HSV`, `SHUFFLE`, `CHROMAKEY`

**Shared work, done once:** a single bind-group-layout shape (one read-only storage buffer in, one storage buffer out, one uniform params buffer) and a single generic dispatch helper in each operation's own file (not shared centrally — see the foundation spec's design principle — but the *shape* is identical enough that writing the first one and copying its structure for the rest is the appropriate level of reuse, not a shared abstraction).

**Per-operation work (small — this is the point of grouping them):**
- `CLAMP`: WGSL body is `clamp(pixel, min, max)` per channel. Params: `min`, `max`.
- `INVERT`: WGSL body is `1.0 - pixel` per channel (check the existing CPU `invert_pixels` for the exact alpha-channel treatment — same convention applies here, don't reinterpret it). No extra params beyond width/height.
- `RGB_TO_HSV`: WGSL body is the RGB→HSV conversion math, ported directly from the existing CPU implementation. No extra params.
- `SHUFFLE`: WGSL body reads the four channel-selector parameters and remaps channels per the existing CPU logic (including the "OFF zeroes a channel" case). Params: four channel selectors.
- `CHROMAKEY`: WGSL body computes color distance from `KEY_COLOR` and produces the keyed alpha, matching the existing CPU math. Params: `KEY_COLOR` (as a vec4 uniform), threshold/tolerance parameters as they exist today.

**Acceptance, per operation:** consume-equivalence-style GPU-vs-CPU numerical-tolerance test (unmasked path only, per the blanket rule), the `is_live()` regression test, `cargo test` passing natively for that operation. Land one at a time — five small, independently-mergeable changes, not one big one.

## Phase 1.2 — Pointwise, zero buffer (procedural): `CHECKERBOARD`, `RING`

No input buffer to upload at all — the shader computes purely from `(x, y)` (via the compute invocation's own `global_id`) and a uniform params buffer. This is a *simpler* bind-group shape than 1.1 (no input storage buffer binding), not a harder one — grouped separately because the pipeline setup genuinely differs, not because it's more work.

- `CHECKERBOARD`: WGSL body ports the existing `((x / tile) + (y / tile)) % 2` checker logic and picks color A or B. Params: `size` (tile), `color_a`, `color_b`.
- `RING`: WGSL body ports the existing ring/radius/spacing/thickness math per ring. Params: whatever `RING`'s current parameter set is (count, radius, spacing, thickness, per-ring color selection) — check `operations/generators/ring.rs` directly for the exact current shape before writing the shader, since it's the most parameter-heavy operation in this phase.

**Acceptance:** same shape as 1.1 (numerical tolerance, `is_live()` regression, native `cargo test`) — these have no `MASK` input at all, so the blanket rule's "masked path untouched" is vacuously satisfied (there's no masked path to preserve), simplifying these two slightly relative to everything else in this document.

## Phase 1.3 — Pointwise, two buffer: `ADD`, `SCREEN`, `SUBTRACT`, `MULTIPLY`, `MIX`, `HUE_KEY`

Same bind-group shape as 1.1 but with two read-only input storage buffers (`Foreground`/`Background`, or `Source`/`Reference` for `HUE_KEY`) instead of one. `MIX` has no `MASK` input (same as the procedural pair above — check `compose/mix.rs` before assuming otherwise) and no Mask input.

**Per-operation work:**
- `ADD`/`SCREEN`/`SUBTRACT`/`MULTIPLY`: WGSL body is each operation's existing per-pixel formula (already written down precisely as `add_single_pixel`/`screen_single_pixel`/etc. from the bbox Phase 3 work — port those directly, they're already the exact per-pixel closures a GPU shader needs, just in Rust instead of WGSL).
- `MIX`: WGSL body is the existing blend-by-parameter math.
- `HUE_KEY`: WGSL body ports the existing hue-comparison keying math against `Reference`.

**Acceptance:** same shape as 1.1/1.2. Land one at a time, six independently-mergeable changes.

## Phase 2 — Resampling: `RESIZE`, `MOVE`

Single input buffer, but the shader computes a *transformed* source address per invocation (center-relative scale for `RESIZE`, a flat offset for `MOVE`) rather than reading its own `(x, y)`. Port the existing `resize_pixels`/`move_pixels` inverse-mapping math directly — both already express "destination → source coordinate" as a pure function, which is exactly what a GPU shader invocation needs (compute my own `(x,y)`, map to a source coordinate, sample there or write transparent if out of bounds).

**Acceptance:** same shape as the phases above. `RESIZE` has no `MASK` input (excluded deliberately, per its own existing doc comment about dimension mismatch — do not add one as part of this phase, that's an unrelated, larger change); `MOVE` does.

## Explicitly not phased

- **`GHOST`** — stateful/temporal, not recommended for this round (see the taxonomy's category 3). If it's ever wanted, it needs its own design pass for how a per-tick history buffer interacts with pipelined GPU dispatch's own cross-tick state — genuinely new territory, not a small extension of the pattern above.
- **`CAMERA`/`IMAGE`/`VIDEO`, `SINE`/`SQUARE`/`LISSAJOUS`, `PATCH`** — not pixel-compute operations at all; not applicable regardless of future GPU work elsewhere.
- **`CROP`** — doesn't exist yet. When it's built, classify it with the taxonomy above (worked example already given) and slot it into whichever pointwise phase is open, rather than opening a phase for it alone.

## Acceptance criteria (whole spec)

1. Each phase/operation lands independently — this document does not require Phase 1.1 through 2 to ship together, or in the listed order, only that each depends on Phase 0 (the operations spec's `BLUR` work) having landed first, since it's what proves the underlying pattern actually works end-to-end before it's replicated nine more times.
2. Every operation's masked/bbox-consuming path (where one exists) is byte-for-byte unchanged — verified by that operation's own pre-existing bbox test suite continuing to pass unmodified.
3. Every operation gets its own `is_live()`-while-`pending` regression test — do not treat this as "already proven by `BLUR`," each operation's own state is independent and needs its own coverage.
4. `cargo test` passes fully and natively after every individual phase/operation lands, not just at the end.
