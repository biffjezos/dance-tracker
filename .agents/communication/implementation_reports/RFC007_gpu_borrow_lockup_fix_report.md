REPORT-ID: REPORT-RFC-007
Created: 2026-08-09
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-webgpu-compute-backend ("Correction — 2026-08-08" appended under "App boot") / RFC-007
Status: Awaiting Code Review

Summary: Fixed the critical `App.gpu` borrow-lockup bug RFC-007
identified: `init_gpu`'s `async fn(&mut self)` held a `wasm-bindgen`
object-borrow of `self` across its `.await`, and a raw JS `TypeError`
escaping mid-poll (adapter/device request throwing directly from
generated glue rather than resolving to a Rust `Result::Err`) left that
borrow permanently stuck, bricking every subsequent call into the `App`
object for the rest of the page's life.

Implemented:

1. `App.gpu`: `Option<Arc<GpuState>>` -> `Rc<RefCell<Option<Arc<GpuState>>>>`.
2. `App::new()`: `gpu: None` -> `gpu: Rc::new(RefCell::new(None))`.
3. `init_gpu`: no longer `async fn(&mut self) -> Result<(), JsValue>`.
   Now a plain, synchronous `fn init_gpu(&self) -> js_sys::Promise`. It
   clones `self.gpu` (a cheap `Rc` clone, not a borrow) into a local, then
   hands that clone - not `self` - into an `async move` block wrapped by
   `wasm_bindgen_futures::future_to_promise`. `self`'s own borrow is only
   held for the synchronous body of `init_gpu` itself (one `Rc::clone`),
   released the instant the function returns the `Promise` - well before
   `GpuState::new()` ever starts polling. On success, the async block
   writes `*gpu_cell.borrow_mut() = Some(Arc::new(gpu))` - ordinary
   interior mutability, entirely orthogonal to `wasm-bindgen`'s own
   external object-borrow tracking. On any failure (clean `Err` via `?`,
   or - per the fix's actual purpose - a raw JS throw escaping mid-`.await`),
   the `RefCell` is simply never written and `gpu` stays `None`; since no
   `wasm-bindgen` borrow of `self`/`App` was ever held during that
   failure, no other `App` method is affected.
4. `App::context()`: `gpu: self.gpu.clone()` -> `gpu: self.gpu.borrow().clone()` -
   a short-lived, synchronous borrow scoped to one statement (nothing
   like the cross-`.await` borrow this RFC fixes). `Context.gpu`'s own
   type (`Option<Arc<GpuState>>`) and value are unchanged from `Context`'s
   perspective.

JS call site (`ui/scripts/app.js`'s `boot()`) is unaffected:
`wasmApp.init_gpu().then(...).catch(...)` still sees a native `Promise`
either way - `future_to_promise` produces the exact same JS-visible shape
`#[wasm_bindgen] async fn` used to, so no JS change was needed or made.

Files modified:

- `engine/src/app.rs` (field type, `App::new()`, `init_gpu`, `context()`)

Architecture notes: Chose `future_to_promise` (a plain `fn` returning
`js_sys::Promise`) over keeping `async fn init_gpu(&self)` sugar, because
Rust's `async fn` desugaring ties the returned future's lifetime to
`self` at the *type* level regardless of whether the body still
references `self` after some point - the exact mechanism RFC-007
diagnoses. A plain function that constructs and returns an `async move`
block capturing only the cloned `Rc` (never `self`) is the structurally
unambiguous way to guarantee no `self`-borrow can ever be part of the
polled future, rather than relying on `self` merely being *unused* late
in an `async fn` body still typed as borrowing `self` for its whole
future.

Tests executed: None new (see "Known limitations" - `app.rs` is
`#![cfg(target_arch = "wasm32")]`-gated and has zero existing tests of
any kind). Reused RFC-005/RFC-006's regression suites conceptually (no
operation code touched, so no operation test should be affected) and
manually traced `init_gpu`'s new borrow lifetime against RFC-007's own
acceptance criteria.

Test results: **Unverified**, for two independent, compounding reasons:

1. `cargo build`/`cargo test` are blocked by sandbox network policy in
   this session (`index.crates.io` 403, "Host not in allowlist"), same
   restriction as RFC-005/RFC-006's precedent.
2. Even with working `cargo`/network access, `app.rs`'s entire content is
   excluded from an ordinary native `cargo test --lib` run by its own
   top-of-file `#![cfg(target_arch = "wasm32")]` - this file has never
   been exercised by this project's native test suite, RFC-007 or not.
   Verifying it requires a `wasm32-unknown-unknown` target build (this
   repo has no `wasm-pack`/wasm32 build script committed anywhere I could
   find - `ui/package.json` only runs a plain Handlebars HTML build) plus
   a real or headless browser to actually execute the compiled module and
   observe `wasm-bindgen`'s runtime borrow-guard behavior, which is a JS-
   engine-level dynamic check, not something `rustc`/`cargo check` alone
   can validate even if dependency resolution succeeded.

In lieu of any of that: verified brace balance and reviewed the diff by
hand; traced through Rust's `async fn` desugaring semantics and
`wasm-bindgen`'s documented object-borrow-guard behavior to justify why
moving the `.await` into an `async move` block that captures only the
cloned `Rc` (not `self`) structurally cannot hold a `self`-borrow open
across any suspension point, unlike the original `async fn(&mut self)`.

Known limitations: No automated or manual in-browser reproduction of
either the original bug or this fix - this sandbox has no browser
environment and (per above) `app.rs` was never reachable by the native
test suite even before this change. RFC-007's own Acceptance Criterion 2
anticipates exactly this ("if the test harness genuinely cannot [simulate
a failure], record that explicitly as a known limitation rather than
skipping silently... prove it manually") - I could not complete even the
manual verification path (call `init_gpu()`, force a failure, confirm
other methods still work) because doing so requires a running wasm32
build in a JS environment, which is unavailable in this sandbox.
Recommend the Code Reviewer or Management verify this fix against a real
browser build before considering RFC-007 fully closed, given its
critical severity and that this is the one acceptance criterion I could
not satisfy even manually.

Specification deviations: None from the required behavior described in
RFC-007. `future_to_promise` vs. keeping `async fn` sugar is a structural
implementation choice within the RFC's own stated flexibility ("either
shape works, as long as no `&mut self`/`&self` `wasm-bindgen` object-borrow
is held across the `.await`"), documented above.

Reviewer notes: This is the third RFC in this dev/reviewer session pair
today (RFC-005, RFC-006, now RFC-007) touching the GPU backend - please
double-check `context()`'s new `self.gpu.borrow().clone()` doesn't
introduce any subtle double-borrow risk if `render_tick`/`preview_tick`
ever call `context()` from inside a scope that already holds
`self.gpu.borrow()` open elsewhere; I traced the current call sites and
found none, but flagging for a second look given `RefCell`'s panic-on-
conflicting-borrow behavior would itself be a new class of app-breaking
bug if introduced here.
