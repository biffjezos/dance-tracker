<!-- .agents/docs/guidelines_and_conventions/ENVIRONMENT_DIAGNOSTICS.md -->
---
title: Environment and Network Diagnostics
owner_role: management
---
# Environment and Network Diagnostics

Use this before concluding that a build/test failure is caused by sandbox
network policy rather than code or specification. Confusing the two wastes
a round trip: an infrastructure restriction cannot be fixed by the
Software Architect (no specification change addresses it) or by changing
the code under test - only Management can reconfigure an environment's
network policy or authorize an offline workaround (see "If a restriction
is confirmed" below).

## Step 1 - check the proxy's own status, don't infer from a single failed request

```
curl -sS "$HTTPS_PROXY/__agentproxy/status"
```

Read two fields:

- `noProxy` - hosts that bypass the proxy entirely (implicitly allowed).
  Package registries are commonly allowlisted here (e.g.
  `index.crates.io`, `registry.npmjs.org`, `pypi.org`,
  `proxy.golang.org`) even when the human-facing website for the same
  service (e.g. plain `crates.io`) is not, and is genuinely blocked
  (403). A blocked website does not imply the registry protocol the
  build tool actually uses is also blocked - check the specific host the
  tool connects to, not the one a human would type into a browser.
- `recentRelayFailures` - if the proxy itself rejected a CONNECT to a
  given host, it appears here with a host, timestamp, and reason (e.g.
  `connect_rejected`, `403`). This is authoritative: a rejection recorded
  here is a policy decision stated in the sandbox's own configuration,
  not something to infer secondhand from a tool's error message.

## Step 2 - test the exact host the failing tool actually uses, not a proxy for it

Different tools/ecosystems use different hosts for the same package. For
Rust/`cargo` and crates.io specifically:

| Host | Used for | Blocking this breaks |
|---|---|---|
| `crates.io` | Human-facing website/API | Nothing `cargo` needs directly |
| `index.crates.io` | Registry index (dependency resolution) | `cargo generate-lockfile`, `cargo update` |
| `static.crates.io` | Actual `.crate` tarball downloads | `cargo fetch`, `cargo build`, `cargo test` |

Test with a plain request first, not a full build (faster to isolate):

```
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://index.crates.io/config.json
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://static.crates.io/crates/<any-crate>/<any-crate>-<version>.crate
```

`HTTP 200` on both means the registry itself is reachable - if `cargo
fetch`/`cargo build` still fails after that, it's a code/dependency
problem, not a network one. A `403`/connection failure on one host but not
the other (as happened investigating the `wgpu` dependency for the
WebGPU compute backend spec - `index.crates.io` open, `static.crates.io`
rejected) means a real, host-specific policy split, not a uniform
"registry is blocked."

The same shape applies to other ecosystems where a single-host
"downloads" API gets mistaken for the whole service (e.g.
`registry.npmjs.org` for npm, `pypi.org`/`files.pythonhosted.org` for
pip) - check the actual download host, not just the index/API host.

## Step 3 - classify and report

- **Both hosts reachable, build still fails** -> code/dependency problem.
  Report normally (implementation report, or an RFI to the Software
  Architect if it's a specification question).
- **A needed host is genuinely blocked** (confirmed via Step 1's
  `recentRelayFailures`, not just a retry) -> infrastructure problem, not
  architecture or code. File a **Notification** (see
  `communication_protocol.md`) addressed to Management, including the raw
  output from Steps 1-2. Do not file this as an RFI to the Software
  Architect - no specification change fixes a network policy.

## If a restriction is confirmed

This is a Management decision (environment configuration, per
`governance_and_organization.md`'s Authority Model), typically one of:

- Reconfigure or recreate the environment with a network policy that
  allowlists the needed host(s).
- Vendor the dependency tree for offline builds (`cargo vendor` plus a
  `.cargo/config.toml` pointing at it, committed to the repo) so the
  build no longer depends on any given sandbox's network policy. Worth
  doing once, proactively, for a dependency added specifically to enable
  a new capability (like `wgpu`), rather than re-diagnosing this per
  sandbox every time a fresh environment is created.

Until resolved, code review and delivery must record the affected
acceptance criteria as **unverified**, not passing.
