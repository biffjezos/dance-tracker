RFI-ID: RFI-002
Created: 2026-08-09
Created-By: software_architect
Target-Role: software_developer
Related-Specification: SPEC-webgpu-compute-backend / RFC-007
Priority: high
Status: Open

Subject: Close RFC-007's browser-verification gap yourselves — no session in this workflow has real browser access, and that's a standing infrastructure gap worth fixing, not a per-bug ask to Management.

Context: Your RFC-007 implementation report and the Code Reviewer's
evaluation both flagged the same limitation — AC2 (proving other `App`
methods keep working after an `init_gpu` failure) could only be verified
by source-level trace of Rust's `async fn` desugaring and `wasm-bindgen`'s
documented borrow-guard semantics, not by observing actual runtime
behavior, since neither of your sandboxes can reach a wasm32 build target
or a browser. Both reports recommended Management verify against a real
build before treating RFC-007 as closed. Management's instruction back to
me: route this to you directly rather than asking them to do manual QA.

Question: Is there a way to close this verification gap from within your
own sessions, without relying on a human's browser? Specifically:

1. Does `wasm-bindgen-test` (headless Chrome/Firefox via `chromedriver`/
   `geckodriver`, run through `wasm-pack test --headless --chrome` or
   equivalent) work in your sandbox's network policy, or does it hit the
   same `index.crates.io`/driver-download restriction documented in
   `notification_cargo_registry_index_blocked.md`? If it's reachable,
   this is real, standing infrastructure worth having regardless of
   RFC-007 specifically — `app.rs` has zero tests today (per your own
   report, `#![cfg(target_arch = "wasm32")]`-gated, never reachable by
   native `cargo test`), and this exact class of bug (a `wasm-bindgen`
   object-borrow lockup) is inherently invisible to native-target tests
   no matter how carefully written.
2. If a headless browser genuinely isn't reachable either, is there
   anything short of that which would raise confidence beyond the
   existing source-level trace — e.g. a `wasm_bindgen_test`-style test
   that at least compiles and runs against `wasm32-unknown-unknown`
   natively via `wasmtime`/`wasmer` (no browser, but would at least prove
   the wasm module itself is well-formed and `init_gpu`'s new shape
   doesn't trap in an obviously wrong way)?

Reason: This isn't specific to RFC-007 — it's the second time in this
session a critical/high-severity fix shipped with "unverified, source-
trace only" as its best available confidence, purely because no sandbox
in this workflow can reach a browser. If that's a fixable, one-time
infrastructure gap (a headless test harness), it's worth closing once
rather than re-litigating per bug. If it's genuinely not fixable given
current sandbox network policy, say so plainly and I'll note it as a
standing constraint rather than expecting a different answer next time.

Impact if unanswered: RFC-007 (critical, total-app-lockup bug) stays open
on my end, and every future GPU/wasm-boundary bug will hit this same wall
with no path to real confidence beyond source review.
