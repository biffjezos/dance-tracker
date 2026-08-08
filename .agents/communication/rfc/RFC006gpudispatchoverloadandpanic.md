RFC-ID: RFC-006
Created: 2026-08-08
Created-By: software_architect
Target-Role: software_developer
Related-Specification: SPEC-webgpu-compute-backend (`.agents/roles/software_architect/docs/specifications/SPECwebgpucomputebackend-1.md`, see its "Correction — 2026-08-08" section)
Priority: high
Status: Open

Severity: High — reported by Management: constant high fan/GPU load and a slower app than before the WebGPU backend shipped, plus intermittent full-app hangs (white screen, "all nodes broken") requiring a reload. Not a visual-correctness bug like RFC-005 — this is app stability and a real performance regression versus the pre-GPU baseline.

Finding:

Two real defects in the pipelined GPU dispatch pattern that every
GPU-backed operation shares (`SPEC-webgpu-compute-backend.md`'s "The
pattern every GPU-backed operation follows" — implemented identically by
hand in all 16 operations below). Root-cause chain, in order:

## 1. Unbounded concurrent GPU dispatch for continuously-changing input

Every operation gates a fresh dispatch on this check (identical in all 16
files):

```rust
let already_pending = self.pending.borrow().as_ref().is_some_and(|p| p.matches(&fingerprint));
if !already_pending {
    // ...dispatch a new GPU job...
}
```

This asks "is *this exact fingerprint* already in flight," not "is *any*
dispatch already in flight." For an input that changes every tick — any
live video source, or any procedural generator with a time-varying
parameter, which given this app's own stated purpose ("realtime video
processing", "procedural visual generation") is the common case, not an
edge case — the fingerprint never repeats. `already_pending` is therefore
always `false`, and a **brand new** GPU dispatch launches on every single
animation frame: fresh buffer allocations (input/output/params/readback —
4 per dispatch), a fresh shader dispatch, a fresh `spawn_local` readback
task — with no cap on how many can be in flight simultaneously. WebGPU
buffer-mapping readback genuinely takes multiple animation frames in a
browser (this is exactly why the pipelined design exists at all — see the
spec's own "one-tick-latency" framing) — so for continuously-changing
input, dispatches pile up faster than they resolve, none of them ever get
consumed before the next tick's fingerprint has already moved past them,
and GPU (and CPU, since the same-tick CPU fallback always runs alongside,
correctly, regardless of this bug) work accumulates without bound for as
long as the input keeps changing. This is the direct cause of the
reported fan/heat/slowdown: real, sustained, ever-growing GPU+CPU work
every frame, for zero benefit (the GPU results are always stale by the
time they'd be usable).

## 2. GPU readback failure panics instead of degrading

`GpuState::new()` (`engine/src/gpu/mod.rs`) documents its own contract
explicitly: adapter/device request "Resolves to `Err` — never panics."
That contract was never extended to buffer mapping. Both
`read_buffer_blocking` and `read_buffer_async` (same file) `.expect()` on
a mapping failure:

```rust
receiver.recv().expect("gpu mapping channel closed before a result arrived").expect("gpu buffer mapping failed");
// ...
let data = slice.get_mapped_range().expect("gpu buffer mapping failed");
```

On `wasm32`, `read_buffer_async` runs inside a detached
`wasm_bindgen_futures::spawn_local` task (see every operation's
`dispatch_gpu`) — no caller anywhere can `.catch()` a panic originating
there; it isn't reachable through any `Result`-returning wasm-bindgen
export. A genuine mapping failure — plausibly a GPU device lost due to
Finding 1's sustained overload, or an ordinary thermal/driver condition —
traps the entire WASM instance with no recovery path. Every subsequent
exported call becomes unreliable for the rest of the session: this is the
direct cause of the reported "entire app and browser hangs (white screen
only), then all nodes broken" — `ui/scripts/engine/render.js`'s render
loop keeps calling `requestAnimationFrame(loop)` regardless (its
`try/catch` around `render_tick`/`preview_tick` catches ordinary JS
errors, not a WASM trap that has already poisoned the instance), so the
app never crashes outright — it just stops doing anything real, which
reads as a hang/white-screen rather than a clean error.

## Possibly related: the originally-reported crash

The trace Management first reported (`Cannot read properties of null
(reading 'requestDevice')`, inside wasm-bindgen's generated
`__wbg_requestDevice_...` glue) is a JS-level `TypeError` from calling
`.requestDevice()` on a `null` `GPUAdapter` — i.e. the browser's own
`navigator.gpu.requestAdapter()` resolved to `null` and something (inside
the `wgpu` crate's webgpu backend, not this codebase's own code — `
GpuState::new()` itself only ever calls `.request_adapter()`/
`.request_device()` through `wgpu`'s own Rust API, which is supposed to
turn a `null` adapter into an `Err` before this codebase ever sees it)
proceeded to call `request_device` on it anyway. This may be a distinct,
upstream `wgpu`-crate edge case independent of Findings 1/2, or it may be
a downstream symptom of the same sustained GPU overload (a browser GPU
process restart/context loss under load leaving a stale non-null adapter
reference that later resolves `null` on retry) — not confirmed either
way. **Not required to fix as part of this RFC** — Findings 1/2 above are
independently confirmed root causes for the fan/slowdown and hang/white-
screen symptoms and must be fixed regardless. If this exact `TypeError`
recurs after Findings 1/2 are fixed, file it as its own RFC/RFI with
repro details (browser, OS, GPU, whether it's reproducible on a cold
start or only after sustained use) — that data will help tell whether
it's the same root cause or a genuinely separate one.

Required Change:

## Fix 1 — cap concurrent in-flight dispatch to one per operation instance

Change every operation's dispatch gate from a fingerprint match to "is
any dispatch pending":

```rust
let has_pending = self.pending.borrow().is_some();
if !has_pending {
    let source = FloatImage::from_value(value, ctx)?;
    self.dispatch_gpu(gpu, fingerprint, source);
}
```

Behavior this produces (verify as part of your implementation plan):
content that changes faster than one dispatch's readback latency stays on
CPU fallback continuously (correct — no perceptible regression versus
CPU-only, since the stale GPU result would never have been usable
anyway); content that holds stable for at least one dispatch's latency
still picks up and reuses the GPU result once it lands, same as today.
Total concurrent GPU resource usage per operation is now bounded to
exactly one in-flight dispatch's buffers, regardless of how fast input
changes.

## Fix 2 — GPU readback failure degrades to CPU fallback, never panics

`read_buffer_blocking`/`read_buffer_async` (`engine/src/gpu/mod.rs`) must
return `Result`/`Option` instead of `.expect()`-ing on a mapping failure.
Propagate that failure back through each operation's `dispatch_gpu` (both
the native blocking branch and the `wasm32` `spawn_local` branch) so a
failed dispatch clears `pending` (allowing a future retry) and leaves
`last_gpu_result` untouched — the operation simply keeps using CPU
fallback for that tick and every tick after, until (if ever) a future
dispatch succeeds. No panic reachable from a GPU failure of any kind, on
either target — this actually satisfies `GpuState::new()`'s own already-
stated "never panics" contract, consistently, rather than only at
adapter/device acquisition.

## Scope — applies to every operation sharing this pattern

All 16 files (confirmed via `grep -rl "already_pending = self.pending.borrow" engine/src`):

- `engine/src/operations/compose/add.rs`
- `engine/src/operations/compose/mix.rs`
- `engine/src/operations/compose/multiply.rs`
- `engine/src/operations/compose/screen.rs`
- `engine/src/operations/compose/subtract.rs`
- `engine/src/operations/generators/checkerboard.rs`
- `engine/src/operations/generators/ring.rs`
- `engine/src/operations/key/chromakey.rs`
- `engine/src/operations/key/hue_key.rs`
- `engine/src/operations/transform/blur.rs`
- `engine/src/operations/transform/clamp.rs`
- `engine/src/operations/transform/invert.rs`
- `engine/src/operations/transform/move_op.rs`
- `engine/src/operations/transform/resize.rs`
- `engine/src/operations/transform/rgb_to_hsv.rs`
- `engine/src/operations/transform/shuffle.rs`

Plus `engine/src/gpu/mod.rs` (Fix 2's actual readback change lives here,
once, for both targets).

**Strong recommendation, not a mandate — your call on structure:** Fix
1's gating logic and Fix 2's failure-propagation are byte-for-byte
identical across all 16 files today (that's exactly how this bug ended up
duplicated 16 times in the first place). Consider extracting the shared
`pending`/`last_gpu_result`/dispatch-gate machinery described in the
spec's "The pattern every GPU-backed operation follows" section into a
single generic helper in `gpu/mod.rs` (e.g. a small `PipelinedDispatch<F,
J>`-shaped type each operation owns one instance of) so this class of bug
can't recur independently in a 17th operation later, and so this fix
lands in one place instead of 16. This is a real structural judgment call
on your side — the spec doesn't mandate a specific shape, only that the
corrected *behavior* (Fixes 1 and 2) is realized in every one of the 16
call sites, however you choose to share the code.

## Testing requirements

- A regression test per operation (or one shared test against the
  extracted helper, if you take the recommended structural route) proving
  Fix 1: simulate several ticks with a changing fingerprint on every tick
  and assert at most one dispatch is ever recorded as pending at a time
  (i.e. `pending` is never overwritten by a second in-flight job before
  the first resolves) — mirror the existing per-operation
  `is_live_returns_true_only_while_pending`-shaped tests already present
  for the pipelined-dispatch mechanism as your template.
- A regression test for Fix 2: simulate a mapping failure (native branch,
  where you control the mock/result path) and assert `execute()` returns
  a valid CPU-fallback result rather than panicking, and `pending` is
  cleared afterward rather than stuck `Some` forever.
- Existing GPU-vs-CPU numerical-tolerance tests for all 16 operations must
  still pass unmodified — this RFC does not change any operation's actual
  compute math, only the dispatch-gating and failure-handling around it.
- Per `ENVIRONMENT_DIAGNOSTICS.md`: if `cargo build`/`cargo test` are
  blocked by sandbox network policy in your session, record that
  explicitly as unverified in your Implementation Report, same as
  RFC-005's precedent — don't let it block filing the report.

Acceptance Condition: Code Reviewer approval per the Implementation
Review Loop, then Management approval (this was reported directly by
Management as a stability/performance bug, same acceptance path as
RFC-004).
