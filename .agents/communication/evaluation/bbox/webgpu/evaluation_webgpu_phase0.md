# Evaluation: WebGPU compute backend, Phase 0 — `gpu` module foundation

**Branch:** `claude/webgpu-phase0-foundation`, based on real `origin/dev` tip `0b070ff` (post-RFC-002)
**Spec:** the corrected `SPECwebgpucomputebackend_1.md` — Phase 0 foundation only, generic `struct Op` pattern documented for future operations, **no operation-specific code in scope**. (Note: an earlier, incorrect spec upload that bundled `BLUR`'s own GPU path into this same document was superseded — evaluated against the corrected one, per your clarification.)

## 0. The situation: genuinely unverified code, and I could not fully verify it either — but I found something concrete anyway

The implementor is explicit and correct that this has never successfully compiled: `static.crates.io` is policy-blocked in this sandbox, and unlike RFC-001 (which could dodge the problem by verifying on a pre-breakage base), `wgpu` compiling *is* the deliverable here — there's no clean fallback base. I independently reproduced this exact failure myself (`curl` against the proxy status endpoint, then a real `cargo build` attempt — identical `403`/`CONNECT tunnel failed` on `static.crates.io`). The implementor also submitted a dedicated RFI response investigating this rather than just asserting it, which I re-verified rather than took on faith.

Given I couldn't build it either, I did the next best thing: **checked the specific wgpu API shapes the implementor explicitly flagged as guesses against the real wgpu 30.0.0 source on GitHub** (`raw.githubusercontent.com`, which — unlike `docs.rs`/`wgpu.rs` — isn't blocked in this sandbox). This is exactly the kind of verification the implementor asked for ("please double check these against a real build") and couldn't do themselves.

## 1. Summary of the change

`GpuState { device, queue }` with generic, operation-agnostic helpers (`create_shader`, `create_compute_pipeline`, `upload`, `create_buffer`, `create_bind_group`, `dispatch`, `copy_buffer_to_buffer`, and target-conditional readback), `Context.gpu`/`App.gpu: Option<Arc<GpuState>>`, and guarded non-blocking boot-time init. Zero operation-specific code — confirmed via diff, no file under `operations/` is touched.

## 2. Verification against requirements

### The wgpu API judgment calls — checked against real source, not guessed at

The report explicitly flagged five places where it filled in wgpu 30.0.0's API shape from indirect evidence (the removed attempt's code, which RFC-001's audit confirmed got *past* dependency resolution) rather than direct documentation access. I fetched the actual `v30.0.0` tag source for each:

| Claim | Verified against real source | Result |
|---|---|---|
| `Adapter::request_device` takes one argument | `pub fn request_device(&self, desc: &DeviceDescriptor<'_>) -> impl Future<Output = Result<(Device, Queue), RequestDeviceError>>` | ✅ Correct |
| `Instance::request_adapter` returns `Result`, not `Option` | `pub fn request_adapter(&self, options: &RequestAdapterOptions<'_,'_>) -> impl Future<Output = Result<Adapter, RequestAdapterError>>` | ✅ Correct |
| `PipelineLayoutDescriptor.bind_group_layouts: &[Option<&BindGroupLayout>]` | `pub bind_group_layouts: &'a [Option<&'a BindGroupLayout>]` | ✅ Correct |
| `PipelineLayoutDescriptor` has `immediate_size`, not `push_constant_ranges` | Confirmed field is `immediate_size: u32`; no `push_constant_ranges` field exists | ✅ Correct |
| **Deliberate deviation:** `BufferSlice::get_mapped_range()` returns a bare `BufferView`, so no `.expect()`/error handling is needed (the implementor removed the reference code's `.expect()` on this basis) | `pub fn get_mapped_range<S: RangeBounds<BufferAddress>>(&self, bounds: S) -> Result<BufferView, MapRangeError>` — quoted verbatim from source | **❌ Wrong** |

Four of five hold up exactly. The fifth — the one place the implementor explicitly *overrode* the reference code with their own judgment rather than keeping it as-is — is incorrect.

### Blocking

- **`get_mapped_range()` is called with no error handling in both `read_buffer_blocking` and `read_buffer_async`, and will not compile as written.** Both functions have:
  ```rust
  let data = slice.get_mapped_range();
  let result: Vec<f32> = bytemuck::cast_slice(&data)[..len].to_vec();
  ```
  Since `get_mapped_range()` returns `Result<BufferView, MapRangeError>` (confirmed above, verbatim from the real wgpu 30.0.0 source), `data` here is a `Result`, not a `BufferView` — `bytemuck::cast_slice(&data)` cannot compile against a `Result`. This affects **both** the native (`read_buffer_blocking`, `#[cfg(not(target_arch = "wasm32"))]`) and wasm32 (`read_buffer_async`) paths identically, since both were written the same way. This means: even once the sandbox's network restriction is resolved, `cargo build`/`cargo test` will **still fail** on native — the one target this spec's own AC1 requires clean — for this reason alone, independent of the network problem. The reference code's original `.expect("Failed to map GPU buffer")` (which the implementor deliberately removed, believing it belonged to a different, non-`Result`-returning API shape) needs to come back — via `.expect(...)` (consistent with the two other `.expect()`s already present in the same functions, on the `map_async` channel/callback result) or, more idiomatically, propagated with `?` if the enclosing function's own return type allows it. This is a two-line fix in `gpu/mod.rs`, but it should be applied and re-verified before this is considered ready, since it's now a known, confirmed compile error rather than an open question.

### Not blocking, but real gaps worth naming precisely

- **AC3's specific named acceptance test is genuinely unwritten** ("after `init_gpu()` resolves in a test/integration context, the *next* `App::context()` call returns `Context.gpu = Some(...)`"). I independently confirmed the implementor's stated reason: `engine/src/app.rs` carries `#![cfg(target_arch = "wasm32")]` at the module level (verified directly — the whole file is gated, not just some items), and `grep -c "#\[test\]"` on it returns `0` — there has never been a native unit test for anything in `App`, this predates this change entirely. I also independently tried to install `wasm-pack`/the `wasm32-unknown-unknown` target myself (to see if a `wasm-bindgen-test` alternative was reachable) and hit the same network restriction. This is a genuine, structurally-forced gap in this sandbox, not corner-cutting — but it means AC3 (as literally worded) is unmet, and someone with real build/wasm-test access needs to close it before this spec can be called fully done, even after the compile fix above lands.
- **Neither of the two authored tests (`gpu_state_new_resolves_to_a_result_without_panicking`, `a_trivial_pipeline_round_trips_through_upload_dispatch_copy_and_readback`) has ever run.** Both are reasonably designed (the second gracefully skips rather than fails when no GPU adapter is present, consistent with `GpuState::new()`'s own "no GPU is an ordinary outcome" philosophy) — but "reasonably designed" isn't the same as "verified," and the second one would hit the `get_mapped_range()` bug above the moment it actually runs.

### Everything else checks out

- **Scope discipline:** confirmed via `git diff --stat` — exactly `gpu/mod.rs` (new), `context.rs`, `app.rs`, `lib.rs`, `Cargo.toml`, `Cargo.lock`, `app.js`, plus the report and RFI markdown files. Zero files under `operations/` touched — matches the corrected spec's explicit "foundation only, no operation implemented" scope precisely.
- **`Context`/`App` wiring:** `Context` still derives `#[derive(Clone, Default)]` (confirmed directly — `Option<T>` is `Default` regardless of `T`, so this was never actually at risk). `App::context()`'s seam adds `gpu: self.gpu.clone()` exactly where the spec names it, and — critically — is a plain `.clone()`, not the removed attempt's panicking `.expect("Compute backend not initialized")`. This is the single most important behavioral fix this phase makes, and it's done correctly.
- **Boot guarding (`app.js`):** fire-and-forget `.then()/.catch()`, never awaited inline before the rest of `boot()` — matches the spec's explicit requirement and directly avoids the removed attempt's boot-blocking bug.
- **Dependency versions:** re-checked the committed `Cargo.lock` directly — `wgpu 30.0.0`, `pollster 1.0.1`, `bytemuck 1.25.2`, `wasm-bindgen-futures 0.4.76` all resolved to exactly the spec's pinned versions, no conflicts. (Minor, inconsequential note: the spec claims `wasm-bindgen-futures` was "already a dependency, predates this work" — that's stale; RFC-001 actually removed it as dead weight. Doesn't affect correctness since it was re-added at the right version regardless, just a spec/reality mismatch worth flagging for whoever maintains these documents.)
- **No blocking calls reachable from wasm32 (AC4):** audited directly — `device.poll(PollType::Wait)` and the blocking `mpsc::Receiver::recv()` both appear exactly once, both inside `read_buffer_blocking`, which is the only function gated `#[cfg(not(target_arch = "wasm32"))]`. `read_buffer_async` never calls `.poll()` and is driven entirely by `map_async`'s own callback via a hand-rolled `Future`. Structurally correct.

## 3. What was done well

- The `.expect()`-vs-`Result` question is the *only* place the implementor deviated from a working reference by their own judgment rather than keeping it verbatim — and it's exactly the one place that turned out wrong. That's not a knock on the overall discipline; if anything it reinforces that keeping close to verified-working reference shapes elsewhere (the other four API questions) was the right call, and the lesson going forward is to keep deviations minimal and flag them prominently, which this report already did well enough that I could go find and check the exact spot.
- The RFI response is a model of how to handle an environmental blocker: reproducible commands, raw output, a clear verdict, and an honest statement of what remains unverifiable rather than papering over it.
- Correctly diagnosed and explained a second, independent structural test gap (AC3's named test) rather than silently omitting it or fabricating an unrunnable test file "for coverage."
- Scope discipline against the corrected spec is exact — no speculative `BLUR`-specific work, matching your own note that no operations should appear in this round.

## 4. Recommendation

**❌ Request changes.** Not because of process (the network blocker is real, independently confirmed, and outside anyone's control here) — but because I found a concrete, verified compile error in the one place the implementor deviated from known-working reference code. Fix `get_mapped_range()`'s error handling in both `read_buffer_blocking` and `read_buffer_async` (two lines), then this needs an actual `cargo build`/`cargo test` pass — from an environment with real `static.crates.io` access — before it can be called done. AC3's test gap should also be tracked explicitly (not silently dropped) for whoever has wasm32 build access next, since it's a real, currently-unmeetable-here acceptance criterion, not an optional nice-to-have.
