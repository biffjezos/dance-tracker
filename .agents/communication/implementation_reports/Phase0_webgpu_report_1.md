# Implementation Report: WebGPU compute backend, Phase 0 — `gpu` module foundation (round 2)

**Branch:** `claude/webgpu-phase0-foundation` (based on real `origin/dev` tip `0b070ff`, post-RFC-002)
**Commit:** `a8fbd08` (fixes the evaluator-caught blocking bug from `de2b595`/`060ec66`)
**Spec:** `SPECwebgpucomputebackend2.md` — Phase 0 foundation only, no operation-specific code

## Fix applied (blocking, from evaluator review of the round-1 submission)

**Bug:** both `read_buffer_blocking` (native) and `read_buffer_async` (wasm32) in `engine/src/gpu/mod.rs` called `slice.get_mapped_range()` with no arguments and no error handling:

```rust
let data = slice.get_mapped_range();
let result: Vec<f32> = bytemuck::cast_slice(&data)[..len].to_vec();
```

This was a deliberate deviation I made from the removed attempt's reference code, which called `.get_mapped_range().expect(...)` — I believed that `.expect()` belonged to a different, non-`Result`-returning API shape and removed it. The evaluator fetched wgpu 30.0.0's real source directly (`raw.githubusercontent.com`, not blocked in this sandbox even though `docs.rs` is) and confirmed the actual signature:

```rust
pub fn get_mapped_range<S: RangeBounds<BufferAddress>>(&self, bounds: S) -> Result<BufferView, MapRangeError>
```

Wrong on both counts: it takes a required `bounds` argument, and it returns a `Result`, not a bare `BufferView`. `bytemuck::cast_slice(&data)` cannot compile against a `Result`. This affected both readback paths identically since both were written the same way.

**Fix:** changed both call sites to:

```rust
let data = slice.get_mapped_range(..).expect("gpu buffer mapping failed");
```

— consistent with the two other `.expect()`s already present in each function, on the `map_async` channel/callback result. Commit `a8fbd08`, `engine/src/gpu/mod.rs` only, 2 lines changed. No other file touched.

All four of the evaluator's other independently-verified API-shape checks (`request_device`'s single argument, `request_adapter` returning `Result`, `bind_group_layouts: &[Option<&BindGroupLayout>]`, `immediate_size` vs. `push_constant_ranges`) were already correct in the round-1 submission and remain unchanged.

## Still open — unchanged from round 1, both structurally forced by this sandbox

- **AC1 (`cargo build`/`cargo test` clean on native) still cannot be verified here.** The network restriction on `static.crates.io` is entirely separate from this bug and remains unresolved (confirmed again via the RFI response I submitted between rounds: `index.crates.io` returns `200`, `static.crates.io` returns `403` at the proxy/CONNECT level, asserted directly in this sandbox's own proxy `noProxy` config). This fix has not been built or tested any more than the rest of the module was in round 1 — I'm relying on the same real-source verification method the evaluator used, not a compiler, to have confidence in it.
- **AC3's named acceptance test is still unwritten**: "after `init_gpu()` resolves in a test/integration context, the *next* `App::context()` call returns `Context.gpu = Some(...)`". `engine/src/app.rs` is `#[cfg(target_arch = "wasm32")]`-gated at the module level with zero pre-existing native `#[test]`s (confirmed again), and no wasm32/browser test runner is reachable in this sandbox — the evaluator independently tried installing `wasm-pack`/the `wasm32-unknown-unknown` target during round 1's review and hit the same network block I would. The wiring itself (`gpu: self.gpu.clone()` inside `App::context()`, set from `init_gpu()`'s own `self.gpu = Some(...)`) is unchanged and directly inspectable, but the acceptance criterion as literally worded remains unmet pending real wasm32 build/test access.

## Everything else

Unchanged from the round-1 report (`.agents/communication/implementation_reports/Phase0_webgpu_report.md`): scope is exactly `gpu/mod.rs` (new) + the `Context`/`App`/`lib.rs`/`Cargo.toml`/`app.js` wiring, zero files under `operations/` touched, `Context` still derives `Default`, `App::context()`'s `gpu` field is a plain `.clone()` (never `.expect()`'d), boot is guarded via a fire-and-forget `.then()/.catch()` in `app.js`, and the two `#[cfg(test)]` tests in `gpu/mod.rs` are unchanged (still unrun, for the same AC1 reason above).
