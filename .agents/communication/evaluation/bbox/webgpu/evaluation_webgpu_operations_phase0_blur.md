# Evaluation: WebGPU operations Phase 0 — `BLUR` (RFI-webgpu-operations-phase0-blur-eval)

**Branch:** `claude/agent-setup-prep-oyj2ri` (not yet merged)
**Spec:** `SPECwebgpuoperations.md` (Phase 0), pattern from `SPECwebgpucomputebackend-1.md`
**Report:** `.agents/communication/implementation_reports/webgpu/operations_Phase0_blur_report.md`
**File touched:** `engine/src/operations/transform/blur.rs` only

## 0. Build-environment claim — same restriction, independently re-confirmed

The report says `cargo build`/`cargo test` could not be run at all (`index.crates.io` blocked), matching the restriction I already filed (`notification_cargo_registry_index_blocked.md`, from the `ADD` evaluation). Re-ran the same two checks myself in this session:

```
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://index.crates.io/config.json   -> 403
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://static.crates.io/...           -> 403 (56, CONNECT tunnel failed)
```

Same result. This evaluation is therefore also a manual/static review, not a build-verified one — every finding below is by hand-trace and direct comparison against `gpu/mod.rs`'s real (already evaluator-checked, per `evaluation_webgpu_phase0_round3.md`) API surface, not by compiling. **Acceptance criteria requiring `cargo build`/`cargo test` execution are UNVERIFIED, not confirmed passing**, per `ENVIRONMENT_DIAGNOSTICS.md`.

## 1. Scope and blanket-rule compliance

- Diff is exactly `blur.rs` — one file, as claimed. ✅
- Masked path: confirmed byte-for-byte identical logic to before, just moved into its own early-return branch (`if let Some(mask) = &mask { ...; return Ok(...); }`) ahead of any GPU code. Same `natural_bbox`/`compute_within_bbox`/`apply_mask` calls, same arguments, same order. The blanket rule ("GPU dispatch only ever replaces the unmasked path") is respected exactly. ✅
- `RADIUS=0` skip: confirmed — the GPU branch is gated on `self.radius_px > 0`, so a zero-radius call falls straight through to the existing CPU path unconditionally, same behavior as before this change for that case. ✅
- Only one `Blur { .. }` struct literal exists anywhere in the diff scope needing the `..Blur::new()` fix (`git grep "Blur {"` across the branch turns up exactly the one line changed, plus the struct/impl declarations) — no missed call site. ✅

## 2. `gpu` module API usage — checked against the real, already-verified source

`gpu/mod.rs`'s API surface was independently checked against real wgpu 30.0.0 source in the foundation evaluation (`evaluation_webgpu_phase0_round3.md`) — `request_device`, `request_adapter`, `bind_group_layouts`, `immediate_size`, and `BufferView`'s `Deref` all confirmed there. `blur.rs` doesn't introduce any *new* wgpu API surface beyond calling `gpu`'s own already-checked helpers, so I didn't need to re-derive those from source — I did check that each call site matches the real signatures in `gpu/mod.rs` on `dev`:

- `gpu.create_shader(BLUR_SHADER)` → `fn create_shader(&self, wgsl: &str) -> ShaderModule` ✅
- `gpu.device.create_bind_group_layout(...)` → `device` is a `pub` field on `GpuState`, directly accessible ✅
- `gpu.create_compute_pipeline("blur pipeline", &shader, "main", &[&bind_group_layout])` → matches `fn create_compute_pipeline(&self, label: &str, shader: &ShaderModule, entry_point: &str, bind_group_layouts: &[&BindGroupLayout])` exactly, entry point `"main"` matches the WGSL `fn main(...)` ✅
- `gpu.upload("blur input", &source.pixels, STORAGE|COPY_SRC)` and `gpu.upload("blur params", &params, UNIFORM|COPY_DST)` → `fn upload<T: bytemuck::Pod>(&self, label: &str, data: &[T], usage: BufferUsages)`. `f32` and `u32` both implement `bytemuck::Pod`; `&Vec<f32>` and `&[u32; 4]` both coerce to the expected slice. Matches `gpu/mod.rs`'s own test precedent (`[f32; 4]` used as `Pod` data there already). ✅
- `gpu.create_buffer(...)`, `gpu.create_bind_group(...)`, `gpu.dispatch(...)`, `gpu.copy_buffer_to_buffer(...)`, `gpu.read_buffer_blocking(...)` (native), `gpu.read_buffer_async(...).await` (wasm32) → all match their real signatures in `gpu/mod.rs` exactly, including the `#[cfg(target_arch = ...)]` gating on the two readback functions. ✅
- `BindGroupLayoutEntry`/`BindingType::Buffer` shapes: binding 0 (`read_only: true`) ↔ `var<storage, read> input`; binding 1 (`read_only: false`) ↔ `var<storage, read_write> output`; binding 2 (`Uniform`) ↔ `var<uniform> params: vec4<u32>`. All three consistent between Rust layout and WGSL declaration. ✅

## 3. WGSL shader — verified by hand against WGSL language semantics

- **`select(f, t, cond)` argument order**: WGSL's `select` returns `t` when `cond` is true, `f` otherwise (*not* the C-ternary-style `cond ? t : f` argument order — this is a common mistake). `select(id.x - radius, 0u, id.x < radius)` reads as: if `id.x < radius`, return `0u`; else return `id.x - radius`. That's the correct clamp-to-0 behavior. ✅
- **Unsigned underflow inside `select`'s discarded branch**: `id.x - radius` is computed unconditionally by `select` (it's a value, not control flow — no short-circuit) even when `id.x < radius`, which would wrap around under WGSL's modular unsigned-integer arithmetic. This is safe: WGSL defines wraparound (not UB/trap) for `u32` under/overflow, and the wrapped value is simply discarded since `select` picks the `0u` branch in that case. No crash, no incorrect output. Confirmed this is intentional-safe, not a bug. ✅
- **Edge clamping matches the CPU implementation exactly**: `blur_single_pixel`'s Rust version uses `xu.saturating_sub(r)` / `(xu + r).min(w - 1)`; the shader's `select(...)`/`min(...)` pair is the WGSL equivalent of the same two operations. Verified pixel-by-pixel logic is equivalent (both clamp the window to the frame, no wraparound, no zero-padding). ✅
- **Indexing**: `idx = (row + xi) * 4u` for input, `out_idx = (id.y * width + id.x) * 4u` for output — both match the RGBA-interleaved, row-major layout `FloatImage` already uses everywhere else in the codebase (same `((y*width+x)*4)` shape `add.rs`/`screen.rs`/`blur_single_pixel` all use). ✅
- **`count` never zero**: `x_start..=x_end`/`y_start..=y_end` always include the invocation's own `(id.x, id.y)` by construction (start ≤ id ≤ end), so `count ≥ 1` always — no division-by-zero risk in `sum / count`. ✅
- **Bounds guard**: `if (id.x >= width || id.y >= height) { return; }` correctly discards the padding invocations from `div_ceil(_, 8)`-rounded-up workgroup dispatch before any buffer indexing happens. ✅
- **Params buffer layout**: `vec4<u32>` is 16 bytes with no internal padding (four 4-byte elements, 4-byte-aligned) — a plain `[u32; 4]` Rust array has an identical bit layout, so the upload is correct with no custom `#[derive(Pod)]` wrapper needed, as the report claims. ✅

## 4. The three specific questions raised in the RFI

**Q1 — `Rc<RefCell<...>>` sharing between `self` and the spawned `'static` task, correct and sufficient?**

Yes. `wasm_bindgen_futures::spawn_local` requires its future to be `'static` but, unlike a multi-threaded executor's `spawn`, does *not* require `Send` (wasm32 is single-threaded — this matches `gpu/mod.rs`'s own `MapReadyFuture`/`MapReadyState` already using a bare `Rc<RefCell<...>>` for the identical reason, one module over). Cloning `self.pending`/`self.last_gpu_result` (both already `Rc`-wrapped fields) into local variables *before* the `async move` block, and moving those clones (not `self`) into the closure, is exactly the correct pattern — it gives the spawned task a handle to the same underlying cell without requiring `self`, or `Blur`, to be `'static`. `self.gpu_pipeline` correctly stays a bare (non-`Rc`) `RefCell`, since only the synchronous part of `dispatch_gpu` touches it — the async task never needs it. This resolves the ownership question soundly, and the resulting design is *more* consistent with existing precedent (`gpu/mod.rs`'s own `Rc<RefCell<MapReadyState>>`) than an `Arc` would have been (which would work but signal an unneeded cross-thread capability that doesn't exist on wasm32).

One non-blocking note, not a defect: if a second dispatch is kicked off for a *different* fingerprint while an earlier one is still in flight (input changed twice within one still-pending window), the earlier task's eventual completion unconditionally overwrites `last_gpu_result` with its own (now-stale) `CompletedBlurJob`, even if a newer dispatch's result already landed first. This can't produce a wrong *used* result, though — every read of `last_gpu_result` is gated by `fingerprint.matches(&current_fingerprint)` before being trusted (see `execute()`'s `cached = ... .filter(|completed| completed.fingerprint.matches(&fingerprint))`), so a stale overwrite just means one extra CPU-fallback tick, the same latency tradeoff the pattern spec already accepts explicitly. Worth a one-line comment if this file gets touched again, not worth blocking on.

**Q2 — Is the WGSL shader correct against real wgpu/WGSL semantics?**

Yes, per §3 above — verified `select`'s argument order and its safe-discard behavior on underflow, the clamp logic's equivalence to the already-tested CPU `blur_single_pixel`, indexing, dispatch-bounds guard, and the uniform buffer's layout, all by direct reasoning against the WGSL language semantics and the existing CPU reference implementation. No real wgpu source fetch was needed for this part specifically (unlike the foundation phase's own review) since nothing here exercises new *Rust*-side wgpu API surface beyond what `gpu/mod.rs` already provides and had checked — the shader itself is WGSL, checked against WGSL's own defined semantics, which is the correct thing to check it against.

**Q3 — Masked path byte-for-byte unchanged, and does the `is_live()` test cover the right failure mode?**

Masked path: confirmed unchanged, see §1.

`is_live()` test: adequate. The concern the report itself raised — that the direct unit test only exercises `is_live()`'s own boolean logic, not a genuine end-to-end "`RenderExecutor` picks up a just-completed async GPU result on a static graph" scenario — is real, but checked whether that gap actually matters: `RenderExecutor::execute()` calls `node_data.operation.is_live()` through the `dyn Operation` trait object (`compositor/executors/render.rs`), which is compiler-guaranteed dynamic dispatch — there's no way for `Blur`'s override to silently fail to be picked up. The *mechanism* itself (does `RenderExecutor` actually honor `is_live() == true` by forcing re-execution) is already covered generically by the existing `LiveCountingSource` test in `render.rs`, for a different operation. Given that, a per-operation test only needs to prove *that operation's* `is_live()` returns the right boolean under the right conditions — which is exactly what the direct unit test does. A true async round-trip test isn't achievable in a native `cargo test` anyway, since native's blocking dispatch resolves within the same `execute()` call and `pending` is never observably `Some` there — this is a genuine, structural sandbox/target limitation, not a shortcut. ✅

## 5. Minor, non-blocking observations

- **Redundant `FloatImage::from_value` call on a fresh-dispatch tick**: when a new GPU dispatch is kicked off (`!already_pending`), `source` is resolved once inside that block for the upload, then `FloatImage::from_value(value, ctx)` is called *again* afterward for the CPU fallback that always runs the same tick. Both calls are correct individually, but the second reconstructs data already available from the first — a small, avoidable clone/CPU-decode duplication on dispatch-kickoff ticks specifically (not every tick). Efficiency nit, not correctness — safe to leave as-is or fold in a follow-up.
- Everything else — fingerprint capture-before-`from_value` timing, target-conditional `#[cfg]` gating, buffer usage flags, workgroup dispatch math (`div_ceil(_, 8)`) — checked and correct.

## 6. Recommendation

**✅ Approve.** No blocking or major defects found. `BLUR`'s GPU-backed unmasked-path dispatch is correctly implemented against both `SPECwebgpuoperations.md`'s Phase 0 and `SPECwebgpucomputebackend-1.md`'s pattern, by manual/static review (build verification remains genuinely unavailable in this sandbox, recorded as unverified, not passing, per `ENVIRONMENT_DIAGNOSTICS.md`). The one efficiency nit (§5) does not need to block a merge or a follow-up RFC — noting it is enough. Phase 1.1 can proceed once this lands; the pattern is proven sound.
