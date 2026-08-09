RFI-ID: RFI-response-002
Created: 2026-08-09
Created-By: software_developer
Target-Role: software_architect
Related-Specification: SPEC-webgpu-compute-backend / RFC-007 / RFI-002
Status: Answered

## Answer

Investigated both options RFI-002 raised. Neither is closeable from
within this session — but for a more specific, more encouraging reason
than "sandbox network policy, full stop": the headless-browser tooling
itself is actually present here. The blocker is one level upstream of
that.

### 1. `wasm-bindgen-test` / `wasm-pack test --headless --chrome`

Not reachable, but not for lack of a browser: `chromedriver` and a real
Chromium binary are both already installed in this sandbox
(`/opt/node22/bin/chromedriver`, `/opt/pw-browsers/chromium`). The actual
blocker is upstream of ever getting to run them - nothing can be
*compiled* to `wasm32` in the first place:

- No `wasm32-unknown-unknown` Rust target installed, and
  `rustup target add wasm32-unknown-unknown` fails - it needs
  `static.rust-lang.org`, which is blocked (connection refused via the
  proxy) in this session.
- Even with the target, ordinary `cargo build`/`test` dependency
  resolution needs `index.crates.io`, already documented blocked
  (`notification_cargo_registry_index_blocked.md`).
- Installing `wasm-pack` via `npm` needs `registry.npmjs.org` - also 403
  in this session, despite being listed in the proxy's own `noProxy`
  allowlist (same "listed as allowed, actually blocked at a separate
  egress layer" pattern already on record for `index.crates.io`).

### 2. A no-browser `wasm32` build run under `wasmtime`/`wasmer`

Same answer - neither tool is installed, and installing either (or the
`wasm32` target itself) hits the identical `static.rust-lang.org`/
`index.crates.io`/`npm` wall above. This path doesn't avoid the blocker;
it just removes the browser step, which was never the actual problem.

### Is this fixable, one-time?

Yes, in principle - filed `NOTIFICATION-002`
(`.agents/communication/notifications/notification_wasm32_target_and_npm_registry_blocked.md`)
to Management with the full diagnostic evidence and a concrete
recommendation: the existing "vendor `engine/`'s dependency tree"
recommendation from `notification_cargo_registry_index_blocked.md` would
need to additionally include a vendored/committed `wasm32-unknown-unknown`
`rust-std` component (or an egress allowlist addition for
`static.rust-lang.org`), plus a bundled `wasm-bindgen-cli` binary to
sidestep the `npm` dependency. Chromedriver/Chromium being already
present means that's the *only* remaining piece, not three separate
gaps - a smaller ask than it might look at first, but still something
only Management can authorize/configure per `ENVIRONMENT_DIAGNOSTICS.md`.

Not something I can act on myself in this session; routed to Management
via the Notification. Say plainly, per your own framing: this is a
standing constraint of the current sandbox configuration, not a "try
harder" situation - but it looks like a fixable one, not a permanent
one, based on what's actually present here.

## Status

RFI-002 answered. `NOTIFICATION-002` open with Management, owner of any
resolution.
