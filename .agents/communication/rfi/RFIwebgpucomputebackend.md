# RFI Packet — WebGPU compute backend spec

**Related Specification:** `.agents/roles/software_architect/docs/specifications/SPECwebgpucomputebackend.md`
**Target Role:** Software Architect
**Created By:** Technical Advisor (relayed via Management)
**Created:** 2026-08-06
**Status:** Open

Note on provenance: these questions were raised by the Technical Advisor during
advisory review with Management, and are being forwarded by Management. Per
the Technical Advisor's role instructions, the Advisor does not file RFIs
directly into the formal workflow — Management owns filing this under
`.agents/communication/rfi/` if it is to become a tracked artifact.

---

## RFI-001

**Subject:** Fingerprint mechanism for `last_gpu_result` matching

**Context:** Design section "`BLUR`'s GPU path", step 2 — "If `last_gpu_result`
matches this tick's actual fingerprint (radius + resolved source pixel
identity) → use it."

**Question:** Is "source pixel identity" meant to be `Arc::ptr_eq` on the
resolved `FloatImage` (matching `RenderExecutor`'s own `value_ptr_eq`
cache-invalidation approach in `compositor/executors/render.rs`), or a
content hash/comparison of the pixel buffer itself?

**Reason:** A content hash or full-buffer comparison computed every tick
would partially defeat the purpose of avoiding per-tick GPU dispatch cost.
Pointer identity is the existing idiom elsewhere in the codebase for
exactly this kind of check.

**Impact if unanswered:** The implementer picks a mechanism arbitrarily.
The wrong choice either adds real per-tick CPU overhead (hashing/comparing
full pixel buffers every tick) or risks false-positive cache hits if
pointer identity is reused across distinct pixel content by some other
part of the pipeline.

---

## RFI-002

**Subject:** `wgpu` crate version and required wasm32 feature flags

**Context:** `wgpu` is a new dependency — not currently present in
`engine/Cargo.toml`. `wasm-bindgen` is pinned at `0.2.126`, `web-sys` at
`0.3.103`. RFC-001 previously removed `wgpu`, `pollster`, and `bytemuck`
from `Cargo.toml` entirely when the prior attempt was reverted.

**Question:** Which `wgpu` version (plus `wasm-bindgen-futures` for the
wasm32 async path, and `pollster` for the native blocking-read path) is
confirmed compatible with the currently pinned `wasm-bindgen`/`web-sys`
versions, targeting the WebGPU backend on `wasm32-unknown-unknown`?

**Reason:** Nothing in the repository currently pins a known-good version
combination for this dependency set.

**Impact if unanswered:** The implementer discovers a version
incompatibility mid-implementation rather than before starting, costing
rework time.

---

## RFI-003

**Subject:** Ownership location of `BlurGpuPipeline` / `PendingBlurJob` /
`CompletedBlurJob`

**Context:** Design section "`BLUR`'s GPU path" shows these as field types
on the `Blur` struct but does not state which module defines them.

**Question:** Do these types belong in `operations/transform/blur.rs`
(operation-owned), or in the `gpu` module?

**Reason:** The spec's own design principle states the shared `gpu`
module stays generic and carries "no operation-specific method" — this
implies `blur.rs` is the intended home, but the spec doesn't say so
explicitly, and this is exactly the kind of detail worth pinning down
given it's the fix for the removed attempt's core mistake (a shared layer
that wasn't actually generic).

**Impact if unanswered:** Ambiguity risks an implementer placing
operation-specific types in the shared module, reproducing the coupling
problem the spec explicitly set out to avoid.

---

## RFI-004

**Subject:** App → Context handoff once `init_gpu()` resolves

**Context:** Design section "App boot" describes kicking off
`App::init_gpu()` via `wasm_bindgen_futures::spawn_local` but does not
describe how a later-resolved `App.gpu` value reaches `Context.gpu` on a
subsequent render tick.

**Question:** Is there an existing per-tick `Context`-construction point
where `App.gpu.clone()` should be read into the new `Context`, or does
this require new plumbing between `App` and wherever `Context` gets built
each tick?

**Reason:** This is the one wiring seam between `App` and `Context` that
Phase 0's stated acceptance criteria don't exercise (they check that the
app boots and `Context` still derives `Default`, not that a resolved GPU
context actually becomes reachable from a live render tick).

**Impact if unanswered:** Phase 0's acceptance criteria could pass in
full while GPU support never actually becomes reachable from a render
tick, due to a missed wiring step discovered only once Phase 1 tries to
consume `ctx.gpu` and finds it's always `None`.
