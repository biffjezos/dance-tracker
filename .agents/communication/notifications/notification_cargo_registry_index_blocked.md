# Notification — `index.crates.io` blocked in this session, not just `static.crates.io`

**Created:** 2026-08-07
**Created-By:** Code Reviewer
**Target-Role:** Management
**Related-Specification:** RFC-002 (ADD Phase 3 migration) — surfaced while evaluating it
**Status:** Open

## Category

Infrastructure / sandbox network policy.

## Symptom

While evaluating the `ADD` bbox-consumption implementation (RFC-002), `cargo build`/`cargo test` could not be run at all in this session — not even against `0b070ff` (the last commit before `wgpu`/`pollster`/`bytemuck` were reintroduced by the WebGPU Phase 0 work), which has no GPU dependencies and previously built cleanly in another evaluator's session.

## Diagnostic Evidence

Followed `ENVIRONMENT_DIAGNOSTICS.md` Steps 1-2 before concluding this was infrastructure rather than a code problem:

```
$ curl -sS "$HTTPS_PROXY/__agentproxy/status"
noProxy: "...,index.crates.io,proxy.golang.org,..."   (index.crates.io explicitly listed)
recentRelayFailures: [{ "host": "static.crates.io:443", "kind": "connect_rejected", "detail": "gateway answered 403 to CONNECT (policy denial or upstream failure)" }]

$ curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://index.crates.io/config.json
HTTP 403
body: "Host not in allowlist: index.crates.io. Add this host to your network egress settings to allow access."

$ curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://static.crates.io/crates/inventory/inventory-0.3.15.crate
curl: (56) CONNECT tunnel failed, response 403
```

`cargo test --offline` also fails immediately (`no matching package named 'inventory' found` — no local registry cache exists at all in this session).

## Verdict

This is a **broader restriction than the one already on record** (`RFIwgpusandboxnetworkcheck.md` / `RFI_response_wgpu_sandbox_network.md`), which confirmed `static.crates.io` blocked but `index.crates.io` reachable (HTTP 200) in that session. Here, `index.crates.io` itself 403s — via a different failure path than the proxy's own CONNECT rejection log (it doesn't appear in `recentRelayFailures` at all; the error text — "Host not in allowlist... network egress settings" — reads as a distinct egress-allowlist layer, not the same proxy-relay denial `static.crates.io` hits). Net effect: dependency *resolution* itself is now blocked, not just `.crate` body downloads — the `SCREEN` evaluation's workaround (resolve deps via `index.crates.io`, build against a pre-`wgpu` commit) is not available in this session either.

This confirms the underlying restriction is **per-session, not fixed** — two sessions evaluating this same repo, days apart, saw different egress policies for the same hosts. Any evaluator relying on the previously-documented "index is fine, only static is blocked" finding as a durable fact will be wrong in some sessions.

## Owner

Management — per `ENVIRONMENT_DIAGNOSTICS.md`, only Management can reconfigure a session's network policy or authorize vendoring the dependency tree for offline builds.

## Resolution

Open. Recommend, if not already planned: vendor `engine/`'s dependency tree (`cargo vendor` + committed `.cargo/config.toml`) so builds stop depending on any given session's egress policy — the existing doc already recommends this "once, proactively" for `wgpu`; this notification is evidence the same instability now also affects the plain, GPU-free dependency set every evaluation needs.

Until resolved: any acceptance criterion requiring `cargo build`/`cargo test` execution must be recorded as **unverified** in evaluations produced from sessions with this policy, per `ENVIRONMENT_DIAGNOSTICS.md`'s closing instruction — this is what I did for the `ADD` evaluation this notification accompanies.
