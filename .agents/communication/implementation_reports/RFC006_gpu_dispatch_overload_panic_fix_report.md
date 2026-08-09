REPORT-ID: REPORT-RFC-006
Created: 2026-08-08
Created-By: software_developer
Target-Role: code_reviewer
Related-Specification: SPEC-webgpu-compute-backend ("Correction — 2026-08-08" section) / RFC-006
Status: Awaiting Code Review

Summary: Fixed both defects RFC-006 identified in the pipelined GPU
dispatch pattern shared by all 16 GPU-backed operations: unbounded
concurrent dispatch for continuously-changing input (Fix 1), and a
readback-mapping failure that could panic and trap the WASM instance
(Fix 2).

Implemented:

## Fix 1 - cap concurrent in-flight dispatch to one per operation instance

In all 16 operations, changed the re-dispatch gate from a fingerprint
match (`already_pending = pending.borrow().as_ref().is_some_and(|p|
p.matches(&fingerprint))`) to "is any dispatch pending"
(`has_pending = self.pending.borrow().is_some()`). A dispatch now only
launches when nothing is currently in flight, regardless of how the
fingerprint has changed - bounding total concurrent GPU resource usage
per operation instance to exactly one in-flight dispatch.

## Fix 2 - GPU readback failure degrades to CPU fallback, never panics

`GpuState::read_buffer_blocking`/`read_buffer_async` (`engine/src/gpu/mod.rs`)
now return `Result<Vec<f32>, String>` instead of `.expect()`-ing on a
mapping failure - extending `GpuState::new()`'s own already-stated "never
panics" contract to buffer mapping, consistently. Every one of the 16
operations' `dispatch_gpu` now matches on that `Result` in both the
native (blocking) and wasm32 (`spawn_local`) branches: on `Err`, clears
`pending` (native: harmless no-op, since `pending` is never set `Some`
before the blocking call; wasm32: clears it if it still matches the
fingerprint that failed, allowing a future retry) and returns without
touching `last_gpu_result`, leaving CPU fallback in control for that tick
and every tick after until a future dispatch succeeds.

## Structural choice: 16 targeted fixes, not an extracted shared type

RFC-006's own text presents extraction into a shared `PipelinedDispatch`-
shaped helper as "a real structural judgment call... not a mandate."
Chose the targeted, per-file fix instead: the change to each file is
small (30 lines), mechanically identical in shape across all 16 (applied
via a script generating the same substitution per file, then verified
individually), and keeps the diff reviewable file-by-file without
introducing a new generic abstraction that would itself need new tests
and carries its own risk of a mismatch against any one operation's
slightly different fingerprint/job-struct shape (compose ops have two
inputs, checkerboard/ring have none, hue_key has an extra Reference,
etc.). This is a deliberate scope choice, not an oversight - happy to
revisit if the Architect or Code Reviewer prefers the extraction.

## Incidental line-ending fix avoided

`blur.rs` and `checkerboard.rs` were the only two of the 16 files using
CRLF line endings (pre-existing, unrelated to this RFC). An earlier pass
of my scripted edit round-tripped them through Python text mode and
silently normalized them to LF; caught this via `git diff --stat` showing
a whole-file rewrite instead of the expected ~30-line diff, and restored
CRLF before committing so the diff for those two files stays as minimal
as the other 14.

Files modified:

- `engine/src/gpu/mod.rs` (Fix 2's actual readback change, plus one new
  regression test)
- All 16 operations RFC-006 named: `engine/src/operations/compose/{add,mix,multiply,screen,subtract}.rs`,
  `engine/src/operations/generators/{checkerboard,ring}.rs`,
  `engine/src/operations/key/{chromakey,hue_key}.rs`,
  `engine/src/operations/transform/{blur,clamp,invert,move_op,resize,rgb_to_hsv,shuffle}.rs`

Architecture notes: No new mechanism, no new field. `pending`'s existing
`Option<Fingerprint>` shape already expresses "at most one job's identity
outstanding" - Fix 1 only changes what question is asked of it
(`.is_some()` instead of a fingerprint-specific match). Fix 2 threads the
existing `Result` idiom already used throughout `gpu/mod.rs` (`GpuState::new()`)
one layer further, rather than introducing a new error type.

Tests executed: `cargo test --lib` targeting the touched modules
(`gpu::tests`, and each of the 16 operations' test modules).

Test results: **Unverified** - `index.crates.io` still blocked in this
session (403, "Host not in allowlist"), same as
`notification_cargo_registry_index_blocked.md` and RFC-005's precedent;
`--offline` has no local registry cache. In lieu of running the suite:

- Verified brace balance and structural correctness of every modified
  file by direct inspection of the generated diff (each of the 16
  operation files' diff is a clean, minimal ~30-57 line change; `gpu/mod.rs`'s
  diff is the `Result`-ification of the two readback functions plus one
  new test).
- Added one new regression test per operation (16 total) proving Fix 1:
  `only_one_gpu_dispatch_stays_pending_while_input_keeps_changing` -
  pre-sets `pending` to a dummy in-flight fingerprint, calls `execute()`
  with a real (adapter-backed, skips gracefully if none available)
  `ctx.gpu`, and asserts `pending` still `.matches()` the original
  fingerprint afterward - proving `execute()` did not launch a second
  dispatch on top of an already-pending one. Mirrors the existing
  `is_live_is_true_only_while_a_gpu_dispatch_is_pending` per-operation
  convention already in every one of these files.
- Added one new regression test in `gpu/mod.rs` proving Fix 2's core
  contract at the layer it actually lives:
  `read_buffer_blocking_on_a_buffer_never_created_with_map_read_returns_err_not_panic` -
  requests a `MapMode::Read` map on a buffer deliberately created without
  the `MAP_READ` usage flag (a deterministic, guaranteed-invalid mapping
  request, not a flaky one) and asserts `Err`, not a panic.

Known limitations:

- No per-operation test forcing a real readback failure through
  `dispatch_gpu` end-to-end (i.e., proving `execute()` itself falls back
  to CPU and clears `pending` when the *operation's own* dispatch fails,
  not just that `gpu/mod.rs`'s readback function returns `Err` in
  isolation). `GpuState` is a concrete struct wrapping real
  `wgpu::Device`/`Queue` with no injectable mock/trait boundary in this
  codebase's current design, so forcing a failure inside one specific
  operation's real dispatch call chain isn't currently possible without
  either tainting production code with a test-only failure hook or a
  larger DI-style refactor - both out of RFC-006's scope. The 16
  operations' `Err`-branch code is simple, structurally identical, and
  doesn't touch anything wgpu-specific (it only clears `pending`/leaves
  `last_gpu_result` alone) - verified by direct reading, not exercised
  end-to-end. Flagging this explicitly per this RFC's own "record rather
  than skip silently" instruction, rather than treating the `gpu/mod.rs`-level
  test as full coverage of the per-operation acceptance criterion.
- `cargo build`/`cargo test` unverified per above.
- Did not investigate the "possibly related" originally-reported
  `TypeError` (`Cannot read properties of null (reading 'requestDevice')`)
  - RFC-006 explicitly marks this out of scope for this round.

Specification deviations: None from the required behavior. Structural
choice (no shared-helper extraction) documented above as an explicitly
allowed judgment call per the RFC's own text.

Reviewer notes: All 16 operation diffs follow an identical shape and were
generated via the same substitution logic, then individually verified
(brace balance, diff review, line-ending preservation) - if one looks
right, the pattern strongly suggests the others do too, but each is
still a real independent file worth a look given a subtle per-file typo
in fingerprint/job-struct naming would be easy to introduce and hard to
notice without compiling.
