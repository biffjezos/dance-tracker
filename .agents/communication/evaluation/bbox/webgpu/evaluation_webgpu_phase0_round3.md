# Evaluation: WebGPU Phase 0 — round 3 fix review

**Commit:** `83bd458` on `claude/webgpu-phase0-foundation`

## The fix is now correct

`slice.get_mapped_range().expect("gpu buffer mapping failed")` — matches `BufferSlice::get_mapped_range(&self) -> Result<BufferView, MapRangeError>` exactly (zero arguments, `Result`-returning). Diffed the commit directly: both call sites changed identically, 2 lines, no other file touched.

Given this is the third pass at the same two lines, and the first two passes each had a real (if narrowing) error, I didn't stop at re-confirming just the flagged call. I checked every other link in the chain and two more non-trivial API surfaces in the same file that hadn't been individually verified yet:

- **`BufferView` → `bytemuck::cast_slice(&data)`:** confirmed `impl Deref for BufferView { type Target = [u8]; ... }` — `&data` coerces to `&[u8]` exactly as the next line assumes. This is the one link in the original chain I hadn't explicitly checked before (I'd verified the method returns `Result<BufferView, _>`, but not what you can actually *do* with the `BufferView` once unwrapped).
- **`BufferSlice::map_async`:** `pub fn map_async(&self, mode: MapMode, callback: impl FnOnce(Result<(), BufferAsyncError>) + ...)` — matches both call sites (`wgpu::MapMode::Read`, a closure receiving `result: Result<(), BufferAsyncError>`) exactly.
- **`PollType::Wait`'s field shape:** confirmed `Wait { submission_index: Option<SubmissionIndex>, timeout: Option<Duration> }` — matches the code's `wgpu::PollType::Wait { submission_index: None, timeout: None }` field-for-field.

Combined with the four API shapes already confirmed correct in round 1 (`request_device`, `request_adapter`, `bind_group_layouts`, `immediate_size`), that's seven distinct wgpu API surfaces in this file now checked directly against the real `v30.0.0` source, all consistent with the code as committed.

## Everything else, reconfirmed unchanged

- Diff scope: `git diff origin/dev...HEAD --stat` — same file set as every prior round plus this report addendum, nothing new touched.
- The scratch-crate experiment (minimal `wgpu`-only crate, same `403` on a different transitive dependency) is a reasonable, well-targeted way to test "is this project-specific or host-level" — consistent with what I'd already independently confirmed twice (direct `curl` to the proxy status endpoint and to `static.crates.io`, plus a real `cargo build` attempt against the actual branch). No need to re-run it myself; it corroborates rather than introduces a new claim.
- AC1 (native `cargo build`/`cargo test`) and AC3 (the `init_gpu()` → `context()` wiring test) remain genuinely unverifiable in this sandbox, for the same structural reasons flagged and independently confirmed across all three rounds. Not a regression, not new.

## On the "citation-based verification" concern the developer raised

Fair concern, and worth answering directly rather than waving off. The two prior misses were both about the *same* nine-character detail (an overload boundary between `Buffer` and `BufferSlice`), not a pattern of broad unreliability — every other API shape checked across all three rounds (seven now) has been correct on the first check. That said, "verified by reading source, not by compiling" is still a categorically weaker guarantee than an actual build, and I'm not asserting otherwise. What I can say with confidence: every line this file's correctness depends on that's checkable this way has now been checked, including the ones adjacent to the two bugs already found, and none of that checking turned up anything else. What I can't do from here is rule out something outside what source-reading can catch (borrow-checker-level lifetime issues across the async/wasm32 boundary, `#[cfg]`-gating interactions, macro-expansion surprises) — that class of problem needs an actual compiler, same limitation as before.

## Recommendation

**✅ Approve, conditional on a real build before merge.** No further issues found after the most thorough pass yet. This should be treated as "ready to merge the moment `cargo build`/`cargo test` can actually run somewhere," not as fully closed — AC1 and AC3 are still open acceptance criteria, not optional ones, and both need real compiler/wasm32 access neither of us has in this sandbox. If that access exists in the other environment the RFI exchange referenced, that's the fastest path to actually closing this phase out.
