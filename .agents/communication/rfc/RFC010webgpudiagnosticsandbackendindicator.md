RFC-ID: RFC-010
Created: 2026-08-09
Created-By: software_architect
Target-Role: software_developer
Related-Specification: SPEC-webgpu-compute-backend (`.agents/roles/software_architect/docs/specifications/SPECwebgpucomputebackend-1.md`, "Correction — 2026-08-09" section — read it in full, this RFC summarizes it)
Priority: critical
Status: Open

Severity: Critical — Management has now reported this exact adapter/device
failure across three rounds (the original report, after RFC-006, after
RFC-007). RFC-007 fixed the app-breaking *consequence* correctly but
never claimed to fix the error itself, which is still an uncaught,
developer-console-visible crash violating this spec's own original
Acceptance Criterion 2 ("zero errors surfaced to the user"). Management
has explicitly instructed: no more partial fixes, no specifications for
features that don't work end-to-end, and comprehensive, honest reporting
of what is and isn't actually fixed.

Finding: See the referenced spec's "Correction — 2026-08-09" section for
the full trace. Summary: `wgpu` 30.0.0's webgpu backend logs "Failed to
create WebGPU Context Provider" (meaning `navigator.gpu.requestAdapter()`
genuinely resolved to `null` in Management's browser) but then still
calls `adapter.request_device(...)` on that `null` result internally,
throwing a raw, unguarded JS `TypeError` that bypasses `wgpu`'s own
`Result`-returning API entirely — `GpuState::new()`'s existing
`.map_err(...)` on `request_adapter()` never gets a chance to see this
failure, because it doesn't surface as an `Err` at all.

**Update, same day, before this RFC reached implementation:** Management
has clarified WebGPU **did** work earlier in this investigation — real
available-feature output in the JS console, and the GPU genuinely under
load (the original fan-noise report that started this whole
investigation). That rules out "this machine never supported WebGPU."
See the spec's own "Correction to this correction" for the full
reasoning — summarized here:

**Three distinct root causes could produce this exact symptom — check
them in this order, not the order they were originally listed in:**

1. **(Check first — fastest, no code required, fits the actual timeline)**
   The browser self-disabled/blocklisted WebGPU for this profile after
   repeated GPU-process instability caused by the original unbounded-
   dispatch overload bug (RFC-006's own root cause, now fixed) — a real,
   documented Chromium self-protection mechanism. **If this is the real
   cause, ask Management to check `chrome://gpu` (or their browser's
   equivalent) for WebGPU's status/blocklist entries and try clearing the
   GPU cache or restarting the browser before writing any code** — this
   may already be the whole fix, with nothing to implement beyond Part B
   below (which is required regardless, as defense-in-depth for other
   users/machines).
2. `Cargo.toml`'s bare `wgpu = "30.0.0"` (no explicit `features = [...]`)
   is missing a fallback backend (e.g. WebGL2) — possible, but a poor fit
   for the observed timeline (would have failed from the very first run,
   not after previously working).
3. Some other browser/machine genuinely has no usable WebGPU adapter at
   all (unsupported hardware/driver/OS, disabled flag, virtualized
   display) — ruled out for Management's own machine specifically (see
   above), but a real category Part B below must still handle correctly
   for any other user.

Required Change:

## Part A — investigate which root cause is real (do this first)

**Start with Scenario 1 above** — ask Management to check `chrome://gpu`
before doing anything else; this needs no code and may resolve the
immediate problem directly. Only if that doesn't explain it: in a session
with real `crates.io`/docs network access (this Architect's own sandbox
does not have it — see `notification_cargo_registry_index_blocked.md`):
run `cargo tree -p wgpu
--edges features` or read `wgpu` 30.0.0's actual `Cargo.toml`/docs to
determine its default feature set for the `wasm32-unknown-unknown`
target, specifically whether a WebGL2 fallback backend is included by
default or requires an explicit feature flag. **Report which scenario (1
or 2 above) is actually true, with the evidence**, in your Implementation
Report — this is a required deliverable, not optional context. If
scenario 2 is confirmed and a feature flag fixes it, add that flag to
`engine/Cargo.toml` as part of this same change.

## Part B — pre-flight adapter check (required unconditionally, regardless of Part A's finding)

Add `web_sys` features `Gpu`, `GpuAdapter` (and `Navigator` if not
already implied by the existing `Window` feature — check) to
`engine/Cargo.toml`. In `crate::gpu::GpuState::new()` (or a thin wrapper
called before it — your call on exact placement), before ever touching
`wgpu::Instance`:

1. Check `web_sys::window().navigator().gpu()`. If `undefined`/`None`,
   return `Err("WebGPU not supported by this browser")` immediately —
   never call into `wgpu` in this branch.
2. If present, call `.request_adapter()` on it directly, await the
   resulting `Promise` via `wasm_bindgen_futures::JsFuture` (same pattern
   `read_buffer_async` in `gpu/mod.rs` already uses for a different
   Promise — follow that precedent). If the resolved value is `null`,
   return `Err("No compatible GPU adapter available")` immediately —
   again, never call into `wgpu` in this branch.
3. Only if this pre-flight check finds a genuine adapter, proceed to the
   existing `wgpu::Instance::request_adapter()`/`request_device()` calls
   unchanged.

This eliminates the uncaught `TypeError` in both scenario 1 and scenario
2-unconfirmed-or-unfixable cases: the failure becomes a clean, controlled
`Result::Err` this codebase produces and can log honestly, not a raw
browser-internals crash. This is required even if Part A finds scenario 2
is fixable and fixes it — real-world browsers without WebGPU support
exist regardless, and this pre-flight check is correct, low-cost
defense-in-depth for all of them, not just Management's current one.

## Part C — status bar backend indicator (Management's explicit direct request)

New `App` method reading the exact same GPU state RFC-007 already
introduced (`Rc<RefCell<Option<Arc<GpuState>>>>>`) — e.g.
`pub fn active_backend(&self) -> String`, returning `"GPU"` if
`self.gpu.borrow().is_some()`, `"CPU"` otherwise. No new state, no new
mechanism.

`ui/scripts/engine/render.js`'s `loop()` reads this every tick (same
shape as its existing `wasmApp.is_output_out_of_gamut()` read) and writes
`"BACKEND: GPU"` / `"BACKEND: CPU"` into the status bar.

**Reuse the status bar's dead `FPS` slot for this, don't add a ninth
field.** `PARKED_WORK.md`'s "RENDER v2" entry (added alongside RFC-008)
already notes `FPS` is slated for removal per Management's own earlier
"currently not working, no real need for it" instruction. Implement that
removal as part of this same change — replace the `FPS` element with the
backend indicator rather than doing two separate passes over the same
line of the status bar template.

Acceptance Criteria:

1. Part A: your Implementation Report states, with evidence, which of
   scenario 1 or 2 is real for a representative test — if you cannot
   determine this (e.g. still no network access in your own session),
   say so explicitly rather than silently skipping it, and Part B/C must
   still ship regardless (they don't depend on Part A's answer).
2. Part B: no uncaught JS error reaches the console on adapter/device
   negotiation failure, on either the "no `navigator.gpu`" path or the
   "adapter resolves null" path — both produce a clean, internally-logged
   `Err` instead. Existing `GpuState::new()` success-path behavior (a
   real adapter/device negotiation) is completely unchanged.
3. Part C: status bar shows `"BACKEND: CPU"` from boot, switches to
   `"BACKEND: GPU"` if and only if a real `GpuState` becomes available,
   and the `FPS` field no longer exists.
4. Existing RFC-005/006/007/008/009 behavior and tests are unaffected —
   this RFC is additive (a pre-check ahead of existing logic, plus a new
   read-only status query) and does not change any operation's dispatch
   or `Recorder` logic.
5. Per established precedent: if `cargo build`/`cargo test` are blocked
   by sandbox network policy, record that explicitly. **Real-browser
   verification is required for this RFC specifically, given the
   critical severity and repeated prior rounds** — do not report this
   closed on source-trace alone; say plainly in your report that
   Management's own machine confirming both "no more uncaught error" and
   "status bar shows the correct backend" is the actual closing
   condition, same as RFC-007's precedent, but do not skip stating it
   this time.

**What this RFC does and does not promise — state this back to Management
in your own report, don't let it get lost:** this guarantees the crash
stops being an uncaught error and guarantees the status bar honestly
shows which backend is active. It does not guarantee GPU becomes active
on Management's specific machine — that depends on Part A's finding,
which may turn out to be a genuine hardware/browser limitation outside
anyone's control. If so, the correct, honest outcome is `"BACKEND: CPU"`
displayed cleanly, not a promise this RFC cannot keep.

Acceptance Condition: Code Reviewer approval per the Implementation
Review Loop, then Management approval — real-browser confirmation
required, per Acceptance Criterion 5.
