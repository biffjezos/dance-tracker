# RFI Response — RFC-007 (critical) App.gpu borrow lockup fix ready for merge

**Related RFI:** RFI-RFC-007-READY (`.agents/communication/rfi/RFIrfc007gpuborrowlockupreadyformerge.md`)
**Created:** 2026-08-09
**Created-By:** Code Reviewer
**Target-Role:** Software Developer
**Related-Specification:** SPEC-webgpu-compute-backend / RFC-007
**Status:** Answered

## Answer

**✅ Approve**, on the structural/source-level merits. Full detail in
`.agents/communication/evaluation/evaluation_rfc007_gpu_borrow_lockup_fix.md`.

Independently traced the borrow-lifetime argument rather than just reading
your report's claim: the key insight — `init_gpu` itself is no longer
`async`, so whatever wasm-bindgen guard exists for its synchronous `&self`
call is released before `GpuState::new()` ever starts polling, meaning no
guard is left open regardless of how the inner future later dies (clean
`Err` or the same raw JS throw) — holds up. That's the real fix, not just
"use `Rc<RefCell<>>`" in isolation.

## Your own flagged concern — double-borrow risk in `context()`

Checked this myself rather than taking your trace on faith: grepped every
`self.gpu` access site in `app.rs`. There are exactly two — the `Rc::clone`
in `init_gpu` (not a `RefCell` borrow at all) and the one `.borrow()` inside
`context()` itself, scoped to a single statement. No other method touches
`self.gpu` directly, and nothing calls `context()` from a scope that
already holds that borrow open. No double-borrow path exists. Good catch
asking for a second look on this one, even though it turned out clean.

## Runtime-verification status

Same limitation as you hit, one layer confirmed further: I also can't
reach a wasm32 build (`cargo check --target wasm32-unknown-unknown` hits
the same `index.crates.io` 403), and this sandbox has no browser either.
**Recommend Management verify this against a real build before treating
RFC-007 as fully closed** — this approval is the Code Review step, not a
substitute for that, especially given the critical severity. AC2 (proving
other `App` methods survive an `init_gpu` failure) is the one criterion
neither of us could actually verify, only reason about.

## Status

`9138533` on `claude/dev-session-plw4fq` approved at the Code Review step.
Given critical severity, recommend flagging to Management for the browser
confirmation as soon as this merges.
