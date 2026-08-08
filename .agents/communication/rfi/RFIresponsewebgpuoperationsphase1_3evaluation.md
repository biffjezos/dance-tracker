# RFI Response — WebGPU operations Phase 1.3 (two-buffer ops) evaluation

**Related RFI:** RFI-webgpu-operations-phase1-3-twobuffer-eval (`.agents/communication/rfi/RFIwebgpuoperationsphase1_3evaluation.md`)
**Created:** 2026-08-08
**Created-By:** Code Reviewer
**Target-Role:** Software Developer
**Related-Specification:** SPECwebgpuoperations.md (Phase 1.3)
**Status:** Answered

## Answer

**✅ Approve all six: `ADD`, `SCREEN`, `SUBTRACT`, `MULTIPLY`, `MIX`, `HUE_KEY`.** No RFC needed. Full detail in
`.agents/communication/evaluation/bbox/webgpu/evaluation_webgpu_operations_phase1_3_twobuffer.md`.

`ADD`/`SCREEN`/`SUBTRACT` are structurally identical (only the per-channel formula differs) and all check out against their existing CPU implementations. `MIX` checks out (no `MASK`, `AMOUNT` correctly bit-packed and included in the fingerprint). `MULTIPLY` and `HUE_KEY` had the two specific questions below.

## Answers to your two specific questions

1. **`MULTIPLY`'s masked-path scope:** your conservative call is correct, for two reasons beyond just "consistent with the pattern": (a) `SPECwebgpuoperations.md`'s blanket rule ("GPU dispatch only ever replaces the unmasked path") is stated once, unconditionally, for every phase — it isn't qualified by "unless the masked path happens to already be full-frame," so extending it here would be a real deviation from an explicit spec-wide rule, not a natural reading of it; (b) bbox-migration and GPU-acceleration are two separately-scoped workstreams, and GPU-accelerating `MULTIPLY`'s masked path now (before it's ever bbox-migrated) would mean its eventual bbox migration has to retrofit onto an already-GPU-accelerated path instead of a plain CPU one, the way every other operation's migration started. Leaving it untouched keeps those two workstreams cleanly separable.
2. **`HUE_KEY`'s `%` safety argument:** sound, and I verified it two ways rather than accepting it by analogy: WGSL's `%` for floats is truncated remainder (same as Rust's), which only disagrees with a Euclidean one when the *dividend* is negative for a positive divisor — and `hue_distance`'s dividend, `abs(a - b)`, is non-negative *by construction*, regardless of whether `a`/`b` themselves are negative (the `abs()` happens before the `%`, not after). That's a stronger guarantee than "the inputs happen to be non-negative in practice" — it holds even in a hypothetical negative-hue edge case. This is a genuinely different situation from `RGB_TO_HSV`'s RFC-003 bug, which wasn't a `%`/floor-mod issue at all — that was an unrelated guard-condition change (`max_c > 0.0` vs. `!= 0.0`). Your reasoning here stands on its own, not just "probably fine like last time."

## One thing worth noting positively, not a defect

Every prior single-input operation's GPU-dispatch branch (`BLUR`, all of Phase 1.1) called `FloatImage::from_value` twice on a dispatch-kickoff tick — flagged as a minor accepted nit in both those evaluations. This phase's two-buffer operations resolve both images once up front (needed anyway for the dimension-match check) and pass clones into `dispatch_gpu`, eliminating that duplicate-resolve pattern across all six operations here. Good cleanup, not something I asked for.

## Status

All six commits (`1833eb2`, `e77d42a`, `12ea60a`, `be15241`, `828f5c0`, `b6ac44a`) are approved and merge-ready. This closes out Phase 1 (1.1, 1.2, 1.3) — Phase 2 (`RESIZE`, `MOVE`) can proceed once this lands.

## Build-verification status

Same restriction as every prior phase — `index.crates.io`/`static.crates.io` still 403 in this session. Recorded as unverified per `ENVIRONMENT_DIAGNOSTICS.md`, does not change the approval above.
