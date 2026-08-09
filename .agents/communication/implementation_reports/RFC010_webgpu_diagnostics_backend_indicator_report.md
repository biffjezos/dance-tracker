REPORT-ID: REPORT-RFC-010
Created: 2026-08-09
Created-By: software_developer
Target-Role: software_architect
Related-Specification: SPEC-webgpu-compute-backend ("Correction — 2026-08-09") / RFC-010
Status: Implemented — merged directly to `dev` per Management's explicit
instruction to review this round in place of the Code Reviewer step.

Summary: Implemented Parts B and C of RFC-010 in full. Part A (determine
whether Management's adapter-null failure is a real hardware/browser
limitation vs. a fixable `wgpu` feature-flag gap) **could not be
completed as specified** — this sandbox has no `crates.io`/docs access,
same restriction documented in `notification_cargo_registry_index_blocked.md`
and every prior RFC in this session. Gathered supplementary real-browser
evidence instead (detailed below) — genuinely useful, but not a
substitute for the `cargo tree -p wgpu --edges features` inspection the
RFC asked for.

## Part A — investigation result: inconclusive from this sandbox, with one real data point

**Could not run** `cargo tree -p wgpu --edges features` or read `wgpu`
30.0.0's actual `Cargo.toml`/docs — `index.crates.io` still 403s in this
session (confirmed again just before starting this RFC). I cannot state
which of scenario 1 (genuine hardware/browser limitation) or scenario 2
(missing `wgpu` feature flag) is true for Management's machine. This is
a real gap in what this report can honestly claim, not something I'm
papering over.

**What I did instead, since this sandbox has a working headless
Chromium** (confirmed useful for RFC-009's pure-JS verification): checked
`navigator.gpu` directly in a real, current browser (Chrome 141) via the
DevTools Protocol, both with default flags and with
`--enable-unsafe-webgpu --enable-features=Vulkan` explicitly set.
**Result: `navigator.gpu` is `undefined` in both cases** — this sandbox's
container has no GPU device passed through to it at all, so the browser
itself never exposes the WebGPU API, regardless of flags. This is a
different (and in a sense more fundamental) failure mode than either
scenario the RFC named: it's not "adapter resolves null" (Management's
actual reported symptom, which implies `navigator.gpu` *does* exist),
it's "the API surface doesn't exist at all." Real, useful confirmation
that Part B's pre-flight check's first branch (`navigator.gpu` itself
undefined) is a genuinely reachable, non-hypothetical condition — but it
tells me nothing about which scenario applies to Management's own
machine, where the symptom (`navigator.gpu` present, `requestAdapter()`
resolving to something that then breaks) is different from what I could
reproduce here.

**Honest conclusion:** Part A remains open. Recommend whoever has real
`crates.io` access (or can run `cargo tree` against a vendored/cached
copy) do the actual feature-flag inspection RFC-010 asked for. I did not
guess or silently assume either scenario.

## Part B — pre-flight adapter check (implemented, unconditional)

`engine/src/gpu/mod.rs`: new `check_web_gpu_available()` (`wasm32`-only,
`#[cfg]`-gated — see below), called as the very first step of
`GpuState::new()`, before `wgpu::Instance` is ever touched:

1. `web_sys::window().navigator()` → reads the `gpu` property. If
   `undefined`/`null`, returns `Err("WebGPU not supported by this
   browser")` immediately.
2. Otherwise, calls `.requestAdapter()`, awaits the `Promise`. If the
   resolved value is `null`/`undefined`, returns
   `Err("No compatible GPU adapter available")` immediately.
3. Only if a genuine adapter is found does `GpuState::new()` proceed to
   `wgpu::Instance::request_adapter()`/`request_device()` unchanged.

**Deliberate deviation from the RFC's suggested implementation shape,
flagged for review:** used `js_sys::Reflect::get`/`js_sys::Function`/
`js_sys::Promise` directly instead of the typed `web_sys::Gpu`/
`GpuAdapter` bindings the RFC suggested adding via new Cargo.toml
features. Reasoning: I have the same `crates.io`/docs access gap Part A
hit — I cannot confirm whether `web_sys` 0.3.103 actually has stable
`Gpu`/`GpuAdapter` bindings, and if so whether `Navigator::gpu()` returns
`Gpu` or `Option<Gpu>` (the WebGPU IDL itself doesn't mark this nullable
— unsupporting browsers simply don't define the property at all, which
is exactly the kind of "undefined behaves unpredictably through a typed
binding" case this whole RFC is about avoiding). `js_sys::Reflect`/
`Function`/`Promise` are long-stable, version-independent `js-sys`
primitives with no such ambiguity, and represent "property doesn't
exist" as a plain `JsValue` I check explicitly, rather than risking a
binding-level conversion failure of their own. Added only the `Navigator`
web-sys feature (needed for `Window::navigator()`) — not `Gpu`/
`GpuAdapter`, since my implementation never constructs those typed
structs. If a future session confirms via real `crates.io` access that
the typed bindings are safe and simpler, switching is a contained,
mechanical change to this one function — not urgent, and not something I
attempted speculatively here.

**`wasm32`-only, matching this file's existing convention**
(`read_buffer_blocking`/`read_buffer_async` are already split the same
way): `web_sys`/`wasm_bindgen_futures` call into JS-host externs that
don't exist to link against on a native build. I caught this myself
before finalizing — an earlier draft called `check_web_gpu_available()`
unconditionally, which would have broken every native `cargo test`
(including every existing GPU-backed operation's own test suite) with a
link failure, a severe regression against RFC-010's own Acceptance
Criterion 4. Gated both the function definition and its call site behind
`#[cfg(target_arch = "wasm32")]`.

## Part C — status bar backend indicator (implemented)

- `engine/src/app.rs`: new `pub fn active_backend(&self) -> String`,
  reading the exact `Rc<RefCell<Option<Arc<GpuState>>>>>` RFC-007 already
  introduced — `"GPU"` if `Some`, `"CPU"` otherwise. No new state.
- `ui/templates/partials/statusbar.hbs` / `ui/index.html` (hand-edited,
  same `npm install`-blocked reason as RFC-008/009): the `FPS` span
  **replaced in place** (same position, `id="backend-indicator"`) rather
  than deleted — deliberately avoiding the index-shift regression class
  RFC-008's `REC` removal required a fix for. `TYPE` (`status.js`'s
  `bar.children[2]`) and `output-resolution` (`output.js`'s
  `bar.children[3]`) are both unaffected by this change; I verified
  their indices don't move.
- `ui/scripts/engine/render.js`'s `loop()`: hoisted the existing
  `const wasmApp = getWasmApp();` (previously scoped inside the
  `isPanelVisible("output")` branch) to the top of `loop()`, and added
  `updateBackendIndicator(wasmApp.active_backend())`, called every tick
  **unconditionally** (not gated on output-panel visibility) — unlike the
  gamut warning, which backend is active is global app state that should
  stay honest regardless of what's currently visible.

Files modified: `engine/Cargo.toml`, `engine/src/gpu/mod.rs`,
`engine/src/app.rs`, `ui/templates/partials/statusbar.hbs`,
`ui/index.html`, `ui/scripts/engine/render.js`.

Tests executed: `node --check` on `render.js` (pass). No Rust tests
added — this RFC adds no new operation/testable unit beyond
`active_backend()` (a two-line `if`) and `check_web_gpu_available()`
(untestable without a real or mocked browser `navigator.gpu`, same class
of gap RFC-006/007 already documented for GPU-failure-injection tests).

Test results: **Unverified — build and runtime, both.** `cargo
build`/`cargo test` blocked (`index.crates.io` 403, established
precedent). No live browser click-through of the actual app possible
either (needs the compiled WASM module). Manually traced:
`check_web_gpu_available()`'s three branches against the RFC's own
required behavior; `active_backend()`'s two-line logic; the status-bar
index-shift question (confirmed no shift, unlike RFC-008's REC case).

**Per RFC-010's own Acceptance Criterion 5, stated plainly, not left
implicit:** this report does **not** claim RFC-010 is closed.
Management's own machine confirming both (a) no more uncaught
`TypeError` in the console, and (b) the status bar correctly shows
`BACKEND: CPU` or `BACKEND: GPU` matching reality, is the actual closing
condition — same as RFC-007's precedent. I have not verified either
outcome myself; I've implemented the change and reasoned through it
carefully, which is a different, lesser thing.

**What this RFC does and does not promise, echoed back per Management's
own instruction not to let this get lost:** this guarantees the crash
stops being an uncaught error, and guarantees the status bar honestly
reflects which backend is active. It does **not** guarantee GPU becomes
active on Management's machine — Part A (which scenario is real) remains
genuinely unanswered from this sandbox. If Management's machine still
shows `BACKEND: CPU` after this lands, that may be the correct, honest
outcome of a real hardware/browser limitation, not a failure of this
implementation — Part A investigation (real `crates.io` access) is the
only way to tell the difference.

Known limitations: Part A open (stated above, not silently skipped).
`check_web_gpu_available()`'s exact `js-sys`/`wasm-bindgen` API usage is
unverified by compilation — reviewed carefully by hand against my
working knowledge of these stable, long-established APIs, but this is
the first RFC in this session where I could not even attempt the
"real headless browser" verification technique RFC-009 used, since this
change requires the actual WASM module to exist to test at all.

Specification deviations: The `js_sys::Reflect`-based implementation
instead of typed `web_sys::Gpu`/`GpuAdapter` bindings, documented and
reasoned above — not a scope change, a documented implementation
judgment call under the same network-access constraint the RFC itself
anticipated.

Merge: Per Management's direct instruction mid-session ("when you are
done with RFC-010 please merge to dev. i review instead of the core
reviewer"), this bypasses the normal Code Reviewer step of the
Implementation Review Loop for this one round — merging directly to
`dev` after this report, not filing an RFI to Code Reviewer. Noting this
explicitly for the record, since it's a deviation from the standard
process documented in `communication_protocol.md`.
