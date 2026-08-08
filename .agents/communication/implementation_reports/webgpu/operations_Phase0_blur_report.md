# Implementation Report: WebGPU operations, Phase 0 — `BLUR`

REPORT-ID: REPORT-webgpu-operations-phase0-blur
Created: 2026-08-08
Created-By: Software Developer
Target-Role: Code Reviewer
Related-Specification: SPECwebgpuoperations.md (Phase 0), depends on SPECwebgpucomputebackend-1.md (Phase 0, already landed on `dev`)
Status: Open

Branch: `claude/agent-setup-prep-oyj2ri`
Files modified: `engine/src/operations/transform/blur.rs`

## Summary

Implements `SPECwebgpuoperations.md`'s Phase 0: `BLUR`'s GPU-backed
dispatch for its **unmasked** path only, per the spec's blanket rule.
`BLUR`'s existing masked/bbox-consuming path
(`compute_within_bbox`/`blur_single_pixel`/`apply_mask`, from the earlier
bbox work) is untouched — same code, same tests, just moved into its own
early-return branch instead of sharing a ternary with the unmasked path.

This is the whole spec's gating phase — per the spec's own acceptance
criterion #1, every later phase (Phase 1.1 through 2) depends only on this
one landing first, "since it's what proves the underlying pattern
actually works end-to-end before it's replicated nine more times." I
scoped this round to Phase 0 alone rather than attempting all ten
operations blind (see "Scope" below) — the remaining phases are each
independently landable once this is reviewed.

## Implemented

- `BlurGpuPipeline` (pipeline + bind group layout, lazily built the first
  time `ctx.gpu` is `Some`), `BlurFingerprint` (source `Value` + `radius_px`,
  compared via `value_ptr_eq` per the pattern spec), `CompletedBlurJob` -
  all operation-owned, defined in `blur.rs`, not the `gpu` module (per
  "each operation owns its own pipeline, shader, and dispatch logic").
- `Blur` gains `gpu_pipeline: RefCell<Option<BlurGpuPipeline>>`,
  `pending`/`last_gpu_result: Rc<RefCell<Option<...>>>`. The latter two are
  `Rc`, not a bare `RefCell` on `self` - the wasm32 async readback task
  (`wasm_bindgen_futures::spawn_local`) needs a handle to the same cell
  that outlives the `&self` borrow of the `execute()` call that spawned
  it, which a bare `RefCell` field can't provide without `self` itself
  being `'static`. This is the one place I had to resolve an ownership
  question the spec states as given ("kick off... via spawn_local, writing
  into pending/last_gpu_result when it resolves") without spelling out how
  a spawned `'static` closure gets a handle back into a `&self`-scoped
  struct - flagging this explicitly in case it should be documented back
  into the pattern spec for the next eight operations, all of which will
  hit the identical question.
- WGSL shader: single-pass 2D window average (not a separable two-pass
  like the CPU path) - `blur_single_pixel`'s own doc comment already
  establishes these are mathematically identical over the same window, so
  this is a correct "naive per-invocation neighbor read" first cut, which
  the spec explicitly allows for Phase 0 rather than requiring workgroup-
  shared-memory tiling.
- Dispatch, per the pattern spec's "Target-conditional dispatch": upload/
  dispatch/copy are ordinary synchronous wgpu calls on both targets;
  readback is the one place they diverge. Native
  (`#[cfg(not(target_arch = "wasm32"))]`) uses `read_buffer_blocking` and
  resolves `last_gpu_result` directly within `dispatch_gpu()` - `pending`
  is therefore never observably `Some` on native in a real run. wasm32
  uses `read_buffer_async` inside a `spawn_local` task, recording
  `pending` until it resolves.
- `is_live()` overridden to `self.pending.borrow().is_some()`, per the
  pattern spec's "Required correctness detail" (a pending GPU dispatch
  must force re-execution so a completed result doesn't get stranded
  behind `RenderExecutor`'s cross-tick cache).
- One pre-existing test's struct literal (`Blur { radius_px: 1 }`) updated
  to `Blur { radius_px: 1, ..Blur::new() }` - a mechanical fix required by
  the new fields, no behavior change.

## Scope

Implemented Phase 0 (`BLUR`) only, not the full ten-operation spec. This
was a deliberate scoping decision, not a shortcut: the spec structures
Phase 0 as the one phase every other phase depends on and nothing else,
explicitly "so it's what proves the underlying pattern actually works
end-to-end before it's replicated nine more times" - and every existing
phase in the codebase's bbox precedent already lands "one at a time" as
independently reviewable units (per the existing per-operation evaluation
files). Given I cannot compile or run any of this in the current sandbox
(see below), landing all ten operations in one unverifiable block seemed
like the wrong risk trade against landing the one gating phase carefully
and letting review catch anything before it's replicated nine times.
Phase 1.1 onward can each follow as their own RFI once this is reviewed.

## Architecture notes

- Blanket rule preserved exactly: GPU dispatch only replaces the
  *unmasked* path. The masked branch now early-returns before any GPU
  code is reached, so there's no interaction between the two at all.
- `RADIUS=0` skips GPU dispatch entirely (falls straight to the existing
  CPU identity short-circuit in `blur_pixels_static`) - not spec-required,
  but avoids a pointless dispatch for a no-op.
- No `Cargo.toml` changes - `wgpu`/`pollster`/`bytemuck`/
  `wasm-bindgen-futures` were all already present from the foundation
  phase. The uniform params buffer uses a plain `[u32; 4]` (matching
  `vec4<u32>`'s 16-byte WGSL layout with no padding) rather than a custom
  `#[derive(Pod)]` struct, specifically to avoid needing to add the
  `bytemuck` `derive` cargo feature - `[u32; 4]`/`[f32; 4]` are already
  relied on as `Pod` via `gpu/mod.rs`'s own existing test.

## Tests executed

None could actually be *run* - see "Known limitations." Written and
intended to run under `cargo test` (native):

- `is_live_is_true_only_while_a_gpu_dispatch_is_pending` - direct unit
  test of `is_live()`'s read-through logic (sets/clears `blur.pending`
  directly via the test module's access to private fields), independent
  of whether a real dispatch is in flight. This is deliberate: native's
  blocking dispatch resolves synchronously, so `pending` is never
  observably `Some` in a real native run - the regression coverage the
  pattern spec requires ("every per-operation phase must include a
  regression test for this specific failure mode") has to be a direct
  test of the method's own logic here, not an end-to-end dispatch test.
- `gpu_blur_matches_cpu_blur_within_tolerance_once_warmed_up` - numerical-
  tolerance GPU-vs-CPU comparison (1e-4 per channel), following
  `gpu/mod.rs`'s own precedent of skipping gracefully
  (`eprintln!("skipping...")`) when no GPU adapter is available in the
  test environment, which this sandbox almost certainly has none of
  (headless container) even before the network block below.
- All of `BLUR`'s pre-existing tests (masked-path/bbox consume-equivalence,
  identity, mismatched-mask-size, etc.) are unchanged apart from the one
  struct-literal fix noted above - none of their logic paths changed.

## Test results

**Could not run `cargo build`/`cargo test` at all.** Confirmed via
`ENVIRONMENT_DIAGNOSTICS.md`'s Steps 1-2 before writing any code:

```
curl -sS "$HTTPS_PROXY/__agentproxy/status"   # noProxy lists index.crates.io explicitly
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://index.crates.io/config.json
  -> HTTP 403, body: "Host not in allowlist: index.crates.io..."
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://static.crates.io/...
  -> curl: (56) CONNECT tunnel failed, response 403
cargo check --lib
  -> error: failed to get `bytemuck` as a dependency ... HTTP 403 on index.crates.io/config.json
```

This matches the Code Reviewer's own `notification_cargo_registry_index_blocked.md`
finding from the `ADD` evaluation - `index.crates.io` is blocked in this
session, not just `static.crates.io` as the earlier-tracked restriction
recorded, so dependency resolution itself fails before a single line of
my own code is compiled. **Acceptance criteria requiring a working
`cargo build`/`cargo test` are UNVERIFIED, not passing** - per
`ENVIRONMENT_DIAGNOSTICS.md`, this is recorded as unverified rather than
guessed at. I did not file a new Notification for this since it's the
same already-tracked restriction Management already has on record from
the `ADD` evaluation; not re-filing a duplicate.

Everything in this report is hand-written and hand-traced against:
`gpu/mod.rs`'s own (evaluator-source-checked) API usage, `blur.rs`'s own
pre-existing code, `ghost.rs`'s `RefCell`/interior-mutability precedent,
and the WGSL language reference for `select`/`vec4<u32>`/uniform-address-
space layout rules. I was not able to independently re-verify any wgpu
API shape against real source the way the Phase 0 foundation's own
evaluator did (`raw.githubusercontent.com` fetches) - I did not attempt
that here since I didn't introduce any new wgpu API surface beyond what
`gpu/mod.rs` already exposes and the evaluator already checked; my own
new code (the WGSL shader itself, the bind group layout shape, the
buffer-usage-flag combinations) has no equivalent external source to
check against and is unverified beyond hand-tracing.

## Known limitations

- No actual GPU adapter or compiler available in this sandbox at any
  point - nothing here has executed even once.
- The wasm32 `spawn_local` path is the first real usage of
  `wasm_bindgen_futures::spawn_local` in this Rust codebase (the existing
  `App::init_gpu()` fire-and-forget happens JS-side, not via
  `spawn_local`) - no prior in-repo precedent to cross-check the call
  shape against beyond the crate's own public API surface.
- Phases 1.1 through 2 (nine more operations) are not implemented - see
  "Scope" above.

## Specification deviations

None identified against `SPECwebgpuoperations.md`'s Phase 0 or
`SPECwebgpucomputebackend-1.md`'s pattern, beyond the ownership question
noted under "Implemented" above (an unspecified-but-resolved detail, not
a deviation from anything the spec stated).

## Reviewer notes

Requesting evaluation per the usual cycle. Since I could not build or
run any of this, please treat every claim above (numerical correctness,
API shapes, WGSL layout/alignment) as needing the same from-scratch
verification the Phase 0 foundation's own evaluation gave `gpu/mod.rs` -
I have no independent confirmation beyond hand-tracing. If review finds
anything not approvable, please respond with an RFC per the usual
process rather than an RFI response, so the required change is explicit
and I can act on it directly rather than needing a follow-up question
round first.
