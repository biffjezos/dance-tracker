# RFI — Confirm whether the wgpu/crates.io fetch failure is a sandbox restriction or a misdiagnosis

**Related Specification:** `SPEC-webgpu-compute-backend.md` (Phase 0)
**Target Role:** Software Developer
**Created By:** Management
**Priority:** High — blocks confirming Phase 0's Acceptance Criterion #1 (`cargo build`/`cargo test` clean on native)
**Status:** Open

## Context

You reported that `static.crates.io` is policy-blocked in your sandbox, and
that no part of `wgpu`'s dependency tree is cached locally, making
Acceptance Criterion #1 unverifiable there.

Independently, in a separate environment, the same exact dependency set
the spec pins (`wgpu = "30.0.0"`, `pollster = "1.0.1"`, `bytemuck =
"1.25.2"`, `wasm-bindgen-futures = "0.4.76"`) was fetched and built
successfully — `cargo fetch` and `cargo build` both completed clean. The
plain `crates.io` website/API was blocked there (403), but that's not the
host cargo actually uses — `index.crates.io` (the registry index) and
`static.crates.io` (the actual `.crate` downloads) both returned 200 and
are explicitly allowlisted to bypass the proxy.

This could mean either (a) the block you saw was checked against the
wrong host and doesn't actually affect cargo, or (b) your specific
sandbox genuinely has a stricter network policy than the one tested here.
We need to know which before deciding how to proceed.

## Question

Please re-run these four commands in your current sandbox and report the
raw output of each:

```
curl -sS "$HTTPS_PROXY/__agentproxy/status"
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://index.crates.io/config.json
curl -sS -o /dev/null -w "HTTP %{http_code}\n" https://static.crates.io/crates/wgpu/wgpu-30.0.0.crate
cargo fetch   # from engine/, with wgpu/pollster/bytemuck/wasm-bindgen-futures in Cargo.toml
```

Then report back:

- Did `cargo fetch` succeed this time? (i.e., was the original failure
  transient or a misdiagnosis — case "resolved")
- Or did it fail again the same way? (i.e., this sandbox genuinely
  restricts these hosts — case "confirmed restricted")

## Reason

This determines whether Phase 0 implementation can simply continue as-is,
or whether a Management-level fix (environment reconfiguration, or
vendoring the dependency tree for offline builds) is needed before
Acceptance Criterion #1 can be met.

## Impact if unanswered

Work stalls on an unverified assumption either way — either effort is
wasted working around a restriction that doesn't actually exist, or
implementation proceeds without ever confirming `cargo build`/`cargo
test` can actually pass, silently failing Acceptance Criterion #1.

---
Management will add a documented rule for this situation (which host to
check, which commands to run, who to escalate to) based on your answer,
so future agents don't have to re-derive this.
