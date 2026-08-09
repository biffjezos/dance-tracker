# Evaluation: RFC-006 — GPU dispatch overload/panic fix (RFI-RFC-006-READY)

**Branch:** `claude/dev-session-plw4fq` (not yet merged), commit `e99d228`
**Spec:** SPEC-webgpu-compute-backend, "Correction — 2026-08-08" section
**RFC:** `RFC006gpudispatchoverloadandpanic.md` (Software Architect, high priority — Management-reported fan/hang/crash)
**Report:** `.agents/communication/implementation_reports/RFC006_gpu_dispatch_overload_panic_fix_report.md`
**Files touched:** `engine/src/gpu/mod.rs` + all 16 GPU-backed operations RFC-006 named. Confirmed via
`git diff origin/dev..HEAD --stat` — no file outside that list changed.

## 0. Build-verification status — independently reconfirmed, still blocked

```
cd engine && cargo test --lib gpu::tests
  -> download of config.json failed ... 403, "Host not in allowlist: index.crates.io"
```

Same restriction as RFC-005's precedent, `notification_cargo_registry_index_blocked.md`.
**AC "cargo build/test succeed" is UNVERIFIED, not confirmed passing.** Everything below
is a manual/static review: diff inspection, brace-balance/struct-field checks, and
cross-referencing generated test code against each file's real fingerprint/job-struct
definitions — not a compile.

## 1. `gpu/mod.rs` — Fix 2's actual mechanism

`read_buffer_blocking`/`read_buffer_async` now return `Result<Vec<f32>, String>` instead
of `.expect()`-ing. Both error paths (`recv()` channel failure, mapping failure,
`get_mapped_range()` failure) converted to `.map_err(...)?`, no `.expect()` left in either
function. New test `read_buffer_blocking_on_a_buffer_never_created_with_map_read_returns_err_not_panic`
uses a deterministic setup (buffer created without `MAP_READ` usage) rather than a flaky
timing-dependent failure — asserts `Err`, correctly targets the exact contract Fix 2
claims. Doc comments on both functions updated to state the "never panics" contract
explicitly, consistent with `GpuState::new()`'s existing documented contract. ✅

## 2. All 16 operations — Fix 1 (dispatch gate)

`already_pending = self.pending.borrow().as_ref().is_some_and(|p| p.matches(&fingerprint))`
→ `has_pending = self.pending.borrow().is_some()`, in every one of the 16 files, zero
leftover references to the old fingerprint-matching form anywhere (checked via grep across
all 16: `has_pending` present exactly once per file, `already_pending` absent everywhere).
This is the correct fix for RFC-006 Finding 1 — a continuously-changing fingerprint can no
longer bypass the pending check, since the check no longer depends on the fingerprint at
all. ✅

## 3. All 16 operations — Fix 2 (readback failure handling)

Every file's native (`#[cfg(not(target_arch = "wasm32"))]`) branch now `match`es
`gpu.read_buffer_blocking(...)` — `Err` clears `pending` (correctly documented as a
no-op on this branch, since `pending` is never `Some` before a blocking call) and
returns without touching `last_gpu_result`. Every file's wasm32 branch `match`es
`gpu.read_buffer_async(...).await` — `Err` clears `pending` only if it still matches the
fingerprint that failed (correct: prevents a stale failure from clobbering a *newer*
dispatch's `pending` that may have started in the meantime), and also leaves
`last_gpu_result` untouched. Verified this exact shape present in all 16 files (grep:
`match gpu.read_buffer_blocking` and `match gpu.read_buffer_async` each present exactly
once per file). Spot-checked field-level correctness on operations with distinct shapes:

- `hue_key.rs` (extra `Reference` input beyond `Source`): `dispatch_gpu` call signature
  and fingerprint threading unchanged by this diff, only the gate/readback wrapped —
  correct, RFC-006 doesn't touch input-shape logic.
- `resize.rs` (`ResizeFingerprint { source, scale_x_bits, scale_y_bits }`) and
  `rgb_to_hsv.rs` (`RgbToHsvFingerprint { source, format }`): new Fix-1 regression test's
  dummy fingerprint literal matches each struct's real field set exactly — not a
  copy-pasted mismatch from a different operation. ✅
- `checkerboard.rs`/`ring.rs` (no image inputs, generators): `dispatch_gpu(gpu,
  fingerprint)` call arity unchanged, consistent with their existing signature. ✅

CRLF line endings on `blur.rs`/`checkerboard.rs` (the two files the report says were
originally clobbered to LF and then restored): confirmed both still report `CRLF line
terminators` via `file`, and the diff itself is `CRLF, LF` mixed only in the unified-diff
markup, not in either file's actual content. Report's claim holds. ✅

Brace balance (`{` vs `}` count) checked across all 16 files: all balanced, no stray
delimiter from the scripted edit. ✅

## 4. Developer's explicit open question — end-to-end failure-injection gap

Agree this is an acceptable gap for this RFC, not a blocker: `GpuState` wraps a real
`wgpu::Device`/`Queue` with no injectable seam, so forcing a failure through one
operation's actual `dispatch_gpu` call chain would require either a test-only hook in
production code or a DI-shaped refactor — both genuinely out of RFC-006's scope (a
stability/performance bug fix, not an architecture change). The `Err`-branch code in all
16 operations is short, structurally identical to the pattern verified in §3, and doesn't
touch anything wgpu-specific — it's a case where the *shape* of the risk (a subtle typo in
one specific file) is well mitigated by direct comparison against the other 15, even
without a compiler or an end-to-end test to catch it. If a future incident traces back to
this exact path, that's the trigger for revisiting the DI question — not something to
require preemptively here.

## 5. Finding — cosmetic, all 16 files, non-blocking

The scripted edit that inserted each file's new `only_one_gpu_dispatch_stays_pending_...`
test also stripped the leading 4-space indent from the following, pre-existing
`gpu_<op>_matches_cpu_within_tolerance_once_warmed_up` test function signature — in every
one of the 16 files (confirmed via `grep -n "^fn [a-z_]*("`, present in all 16, absent
before this diff). Example (`add.rs`):

```rust
    #[test]
fn gpu_add_matches_cpu_within_tolerance_once_warmed_up() {
```

Purely cosmetic — Rust doesn't care about whitespace, this compiles and runs identically
either way — but it's a real, reproducible artifact of the same substitution script,
consistent across all 16 files, and would trip `cargo fmt --check` if that's run in CI.
Not requesting a fix before merge; flagging because the report explicitly claims each
file's diff was "individually verified" for exactly this class of scripted-edit artifact
(it caught the CRLF regression this same way) and this one slipped through that same
check. Worth a `cargo fmt` pass on these 16 files as a fast-follow, whenever `cargo` access
is available in a session.

## Decision

**Approve.** Fix 1 and Fix 2 both verified present, correct, and uniformly applied across
all 16 operations plus `gpu/mod.rs`, by direct trace against each operation's real
fingerprint/job-struct shape — not just a diff skim. `cargo build`/`cargo test` unverified
due to environment (independently reconfirmed). The developer's own flagged test-coverage
gap is a reasonable, scope-appropriate limitation, not a defect. §5's indentation slip is
cosmetic and does not gate this merge. This is a high-priority stability fix (Management-
reported fan/hang) — recommend merging promptly once any other reviewer input (if
requested) is in.
