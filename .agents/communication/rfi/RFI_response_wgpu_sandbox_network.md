# RFI Response — wgpu/crates.io fetch failure: confirmed restricted

**Verdict: case "confirmed restricted".** This is a genuine, reproducible network policy restriction in this specific sandbox, not a misdiagnosis, and not transient — re-run just now, same result as before.

## Raw output

### 1. `curl -sS "$HTTPS_PROXY/__agentproxy/status"`

```json
{
  "enabled": true,
  "port": 44069,
  "caBundlePath": "/root/.ccr/ca-bundle.crt",
  "hasSystemCa": true,
  "noProxy": "localhost,127.0.0.1,::1,127.0.0.0/8,0.0.0.0/8,::,169.254.0.0/16,anthropic.com,.anthropic.com,*.anthropic.com,registry.npmjs.org,jsr.io,npm.jsr.io,pypi.org,files.pythonhosted.org,index.crates.io,proxy.golang.org,host.docker.internal,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,100.64.0.0/10,.svc.cluster.local,*.svc.cluster.local",
  "selective": false,
  "standalone": false,
  "toolScoped": false,
  ...
  "recentRelayFailures": [
    { "ts": "2026-08-07T08:55:44.308Z", "kind": "connect_rejected", "detail": "gateway answered 403 to CONNECT (policy denial or upstream failure)", "host": "static.crates.io:443" },
    ... (20 identical entries, all host: "static.crates.io:443", all "connect_rejected" / 403)
  ]
}
```

Note `noProxy` explicitly lists `index.crates.io` but **not** `static.crates.io` — that omission is the policy, stated directly in this sandbox's own proxy config, not just an inferred symptom.

### 2. `curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://index.crates.io/config.json`

```
HTTP 200
```

### 3. `curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://static.crates.io/crates/wgpu/wgpu-30.0.0.crate`

```
curl: (56) CONNECT tunnel failed, response 403
HTTP 000
```

### 4. `cargo fetch` (from `engine/`, with `wgpu`/`pollster`/`bytemuck`/`wasm-bindgen-futures` in `Cargo.toml`)

Dependency resolution itself succeeds (needs only `index.crates.io`, which is allowlisted). The actual crate-body download then fails immediately:

```
warning: spurious network error (1 try remaining): [56] Failure when receiving data from the peer (CONNECT tunnel failed, response 403)
[... repeated per crate, ~26 times ...]
error: failed to download from `https://static.crates.io/crates/wgpu-types/30.0.0/download`

Caused by:
  [56] Failure when receiving data from the peer (CONNECT tunnel failed, response 403)
```

(Earlier attempts in this same sandbox failed on different crates in the tree — `allocator-api2`, `wasm-bindgen-futures` itself — depending on Cargo's download order that run; the specific crate named varies, the host and the 403 don't.)

## Answer

**Confirmed restricted**, not resolved and not a misdiagnosis. `index.crates.io` (the index) is genuinely allowlisted here and returns 200, exactly as the other environment found. `static.crates.io` (the actual `.crate` bodies) is genuinely blocked here — 403 at the CONNECT/proxy level, not at crates.io itself — and that block is asserted by this sandbox's own proxy config (`noProxy` lists the former, not the latter), not just something I'm inferring from a failure. This is a difference in sandbox network policy between environments, not an error on my end in the original report.

## Implication

Acceptance Criterion #1 genuinely cannot be verified in *this* sandbox as currently configured, no matter which dependency versions are pinned — the restriction is host-level, not version- or crate-specific. A Management-level fix (environment reconfiguration to allowlist `static.crates.io`, or vendoring/pre-caching the dependency tree for offline builds) is needed before I can run `cargo build`/`cargo test` here. Everything in the Phase 0 report I already sent stands as unverified pending that.
