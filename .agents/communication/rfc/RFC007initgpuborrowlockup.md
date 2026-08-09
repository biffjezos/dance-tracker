RFC-ID: RFC-007
Created: 2026-08-08
Created-By: software_architect
Target-Role: software_developer
Related-Specification: SPEC-webgpu-compute-backend (`.agents/roles/software_architect/docs/specifications/SPECwebgpucomputebackend-1.md`, "App boot: guarded, non-blocking, never fails startup" section)
Priority: critical
Status: Open

Severity: Critical — the entire app permanently stops responding to any interaction (cannot add operations, cannot load videos, nothing works) after a single WebGPU init failure, for the rest of the page's life. Requires a full reload to recover, and recurs immediately on reload for any browser/machine where adapter/device request fails this way (confirmed still happening on `dance_tracker_engine.js` built from `c948baa`, i.e. after RFC-006 landed — RFC-006 never touched this path, so this was never actually fixed by it).

Finding:

Management reported, in this order, across one investigation:

1. `Uncaught TypeError: Cannot read properties of null (reading 'requestDevice')` at `__wbg_requestDevice_ab46d0519ea1cc34` — still occurring after RFC-006.
2. `Uncaught Error: recursive use of an object detected which would lead to unsafe aliasing in rust` from `App.execute_operation`.
3. The same "recursive use" error from `App.create_node`, while loading a video (`createLiveSourceNode` → `wasmApp.create_node(operationId)` in `ui/scripts/features/video.js`).
4. "everything is broken" / "i cannot load videos."

These are one causal chain, not three unrelated bugs:

**`App::init_gpu` (`engine/src/app.rs`) is `pub async fn init_gpu(&mut self)`.**
Rust's `async fn` desugaring captures `&mut self` for the *entire* function body — including across the internal `.await` on `crate::gpu::GpuState::new()`, which itself awaits `adapter.request_device(...)`. `wasm-bindgen` enforces Rust's aliasing rules on any JS-callable struct dynamically, at the JS/wasm boundary: while this `&mut self` borrow is checked out (i.e., for the whole span of `init_gpu`'s in-flight Promise, including every suspended `.await` point inside it), no other call into any method on the same `wasmApp` object handle is allowed to proceed — this is exactly what "recursive use of an object detected... unsafe aliasing" is: `wasm-bindgen`'s own correctly-functioning safety net refusing a second borrow of an object it believes is still checked out.

The `requestDevice`-on-null failure (Finding 1) is a **raw, unguarded JS `TypeError`** — thrown directly from generated glue (`getObject(idx).requestDevice(...)` where `getObject(idx)` is `null`), not routed through any Rust `Result`/`?` or through `wasm-bindgen`'s own `__wbindgen_throw` mechanism at all (confirmed: Finding 1's trace does **not** go through `__wbg___wbindgen_throw_...`, unlike Findings 2/3, which do — that function is `wasm-bindgen`'s own deliberate "signal a JS-visible error" path, and its presence in 2/3 but absence in 1 is exactly what distinguishes "a real Rust-detected condition, correctly reported" from "a raw JS exception escaping mid-poll with no Rust-side handling at all"). When that raw exception fires while `init_gpu`'s Future is being polled (i.e., mid-`.await`, inside `GpuState::new()`), it unwinds through the JS/wasm boundary abnormally — bypassing the normal poll-return code path that would otherwise release `wasm-bindgen`'s borrow-guard on `self` once the Future resolves (successfully or via a clean `Result::Err`). The guard is left stuck "borrowed," permanently, because nothing ever completes the async method normally again.

Every subsequent call into `wasmApp` — `execute_operation`, `create_node`, `render_tick`, `preview_tick`, everything — then immediately fails with the same "recursive use" error, because `wasm-bindgen` correctly refuses a second borrow of an object it still believes is checked out. This is Findings 2, 3, and 4: not new, independent bugs, but the inevitable downstream consequence of Finding 1 firing during `init_gpu`'s active `&mut self` borrow.

This also means the design intent already documented in
`SPEC-webgpu-compute-backend.md`'s "App boot" section — "a GPU init
failure... must never block or break app startup" — is not actually true
today: it holds for the *ordinary* failure path (`GpuState::new()`
returning a clean `Result::Err`, which `init_gpu`'s own `?` operator
handles fine, completing the async fn normally and releasing the borrow),
but not for this raw-JS-throw failure mode, which the spec's original
design didn't anticipate.

Required Change:

**`App.gpu` must never be part of a `wasm-bindgen`-guarded borrow held
across an `.await`.** Restructure so `init_gpu`'s actual GPU negotiation
work never touches `&mut self` at all — mirroring the exact `Rc<RefCell<...>>>`
interior-mutability pattern every GPU-backed operation already uses for
its own cross-tick state (`pending`/`last_gpu_result` in `blur.rs`,
`resize.rs`, etc. — this is established precedent in this codebase, not a
new idiom):

1. Change `App.gpu: Option<Arc<GpuState>>` to `App.gpu: Rc<RefCell<Option<Arc<GpuState>>>>`.
2. `init_gpu` clones the `Rc` out (a cheap, non-borrowing operation) and
   hands that clone to a plain `wasm_bindgen_futures::spawn_local` block
   (or keeps `init_gpu` as the `#[wasm_bindgen]`-exported async method,
   but restructure its body so the actual `GpuState::new().await` happens
   in an inner `async move` block that only captures the cloned `Rc`, not
   `self`) — either shape works, as long as no `&mut self`/`&self`
   `wasm-bindgen` object-borrow is held across the `.await`.
3. On success, write into the `RefCell` directly (`*gpu_cell.borrow_mut() = Some(Arc::new(gpu))`)
   — ordinary Rust interior mutability, entirely orthogonal to
   `wasm-bindgen`'s own external object-borrow tracking, so nothing here
   can ever leave that separate JS-visible guard stuck.
4. On failure — of any kind, whether a clean `Result::Err` from
   `GpuState::new()` or a raw JS throw escaping mid-`.await` — the
   `RefCell` simply never gets written, `gpu` stays `None`, exactly the
   already-intended "stay on CPU" behavior. Critically: since no
   `wasm-bindgen` object-borrow was ever held during the failure, no
   other method on `wasmApp` is affected. `execute_operation`,
   `create_node`, `render_tick`, etc. keep working normally regardless of
   what `init_gpu` does or how it fails.
5. `App::context()` (currently `gpu: self.gpu.clone()`) becomes
   `gpu: self.gpu.borrow().clone()` — same `Option<Arc<GpuState>>`
   value handed to `Context.gpu`, unchanged from `Context`'s own
   perspective; only how `App` stores it changes.

**Note on the raw JS throw itself:** this fix does not necessarily
silence Finding 1's console error — depending on how `wasm_bindgen_futures::spawn_local`
handles an exception escaping mid-poll of the block doing `GpuState::new().await`,
the same `TypeError` may still print to the console. That's expected and
acceptable for this RFC: the actual, severe bug being fixed is the
permanent app breakage (Findings 2-4), not the console message itself.
If the console error is still visually noisy after this lands and is
worth silencing on its own, that's a separate, much lower-severity
follow-up (e.g. a defensive `web_sys`-based adapter-availability
pre-check before ever calling into `wgpu`) — file it separately if still
wanted once this fix is confirmed to have stopped the app-breaking
cascade.

Acceptance Criteria:

1. `App.gpu` is `Rc<RefCell<Option<Arc<GpuState>>>>`; no `wasm-bindgen`
   object-borrow of `self` is held across any `.await` point anywhere in
   `init_gpu`'s call graph.
2. On a machine/browser where `GpuState::new()` fails in any way
   (ordinary `Result::Err`, or a raw JS throw escaping mid-`.await`),
   every other `App` method (`execute_operation`, `create_node`,
   `render_tick`, `preview_tick`, at minimum) continues to work normally
   afterward — write a test proving this if the test harness can simulate
   or force an `init_gpu` failure; if it genuinely cannot (no adapter mock
   seam exists, similar to RFC-006's own documented test-coverage gap),
   record that explicitly as a known limitation rather than skipping
   silently, and instead prove it manually: call `init_gpu()`, force/await
   a failure, then call `execute_operation`/`create_node` and confirm no
   panic/no `wasm-bindgen` borrow error.
3. `App::context()`'s `Context.gpu` value is unaffected by this
   refactor — still `Some(Arc<GpuState>)` after a successful `init_gpu`,
   still `None` before/after a failed one, from any other method's
   perspective.
4. Existing tests for every GPU-backed operation (RFC-005/RFC-006's own
   regression tests included) continue to pass unmodified — this RFC only
   changes how `App` stores/hands out its `gpu` handle, not `Context`'s
   shape or any operation's own dispatch logic.
5. Per `ENVIRONMENT_DIAGNOSTICS.md`/RFC-005/RFC-006 precedent: if
   `cargo build`/`cargo test` are blocked by sandbox network policy,
   record that explicitly as unverified rather than skipping the report.

Acceptance Condition: Code Reviewer approval per the Implementation
Review Loop, then Management approval — this was reported directly by
Management as a critical, app-breaking bug.
