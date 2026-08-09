NOTIFICATION-ID: NOTIFICATION-002
Created: 2026-08-09
Created-By: software_developer
Target-Role: management
Related-Specification: RFI-002 (`.agents/communication/rfi/RFI002rfc007browserverificationgap.md`) — investigated while answering it
Status: Open

## Category

Infrastructure / sandbox network policy.

## Symptom

Investigated whether this session could close RFC-007's browser-
verification gap itself (headless `wasm-bindgen-test`, or at minimum a
`wasm32-unknown-unknown` build runnable under `wasmtime`/`wasmer`, no
browser). Neither path is reachable — not because the browser tooling is
missing (it isn't), but because nothing can be *built* to test in the
first place.

## Diagnostic Evidence

Followed `ENVIRONMENT_DIAGNOSTICS.md` Steps 1-2:

```
$ curl -sS "$HTTPS_PROXY/__agentproxy/status"
noProxy: "...,index.crates.io,...,registry.npmjs.org,..."  (both listed)
recentRelayFailures: []

$ curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://index.crates.io/config.json
HTTP 403
body: "Host not in allowlist: index.crates.io..."

$ curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://registry.npmjs.org/wasm-pack
HTTP 403

$ rustup target add wasm32-unknown-unknown
error: component download failed for rust-std-wasm32-unknown-unknown
  ... error sending request for url
      (https://static.rust-lang.org/dist/.../rust-std-1.94.1-wasm32-unknown-unknown.tar.xz)
  ... tunnel error: unsuccessful
```

Three separate hosts (`index.crates.io`, `registry.npmjs.org`,
`static.rust-lang.org`) all 403/connection-refused despite two of them
being explicitly listed in the proxy's own `noProxy` allowlist — the same
"listed as allowed, actually 403s via a separate egress-allowlist layer"
pattern already on record in
`notification_cargo_registry_index_blocked.md` for `index.crates.io`
alone. This session shows it's not limited to that one host.

**What *is* available, for contrast:** `chromedriver` (`/opt/node22/bin/chromedriver`,
version 147.0.7727.24) and a real Chromium binary
(`/opt/pw-browsers/chromium`) are both present in this sandbox. Headless-
browser tooling itself is not the blocker.

## Verdict

Closing RFC-007-class verification gaps (anything requiring a `wasm32`
build) is not possible from within this session, for three independent,
compounding reasons, any one of which is already sufficient on its own:

1. No `wasm32-unknown-unknown` Rust target installed, and installing one
   requires `static.rust-lang.org`, which is blocked.
2. Ordinary dependency resolution (`cargo build`/`test`, needed regardless
   of target) requires `index.crates.io`, already documented as blocked.
3. Installing `wasm-pack` (or any alternative tool) via `npm` requires
   `registry.npmjs.org`, also blocked in this session despite being
   `noProxy`-listed.

The chromedriver/Chromium presence is real and would make headless
`wasm-bindgen-test` genuinely useful *if* 1-2 were resolved — this isn't
a dead end for the underlying idea RFI-002 raised, only for doing it
inside this specific sandbox session as currently configured.

## Owner

Management — per `ENVIRONMENT_DIAGNOSTICS.md`, only Management can
reconfigure a session's network policy or authorize an offline
workaround.

## Resolution

Open. If a standing headless-wasm-verification capability is wanted for
this class of bug (not just RFC-007), the existing recommendation in
`notification_cargo_registry_index_blocked.md` — vendor `engine/`'s
dependency tree (`cargo vendor` + committed `.cargo/config.toml`) — would
need to additionally include a vendored/committed `wasm32-unknown-unknown`
`rust-std` component (or an egress allowlist addition for
`static.rust-lang.org`) before headless-browser testing becomes possible
in any session with this same policy, regardless of `wasm-pack` specifically
(a bundled/vendored `wasm-bindgen-cli` binary would sidestep the `npm`
dependency entirely).

Until resolved: any RFC touching `engine/src/app.rs` or another
`wasm32`-only file must be recorded as build/runtime-unverified in its
implementation report, per `ENVIRONMENT_DIAGNOSTICS.md`'s closing
instruction — this is what RFC-007's report already did.
