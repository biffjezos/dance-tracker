# Implementation Report: WebGPU compute backend, Phase 0 — `gpu` module foundation (round 3)

**Branch:** `claude/webgpu-phase0-foundation` (based on real `origin/dev` tip `0b070ff`, post-RFC-002)
**Commit:** `83bd458` (fixes the evaluator-caught overload mismatch from round 2's `a8fbd08`)
**Spec:** `SPECwebgpucomputebackend2.md` — Phase 0 foundation only, no operation-specific code

## Fix applied (blocking, from round-2 evaluation)

Round 2's evaluator caught that my round-2 fix cited the wrong `get_mapped_range` overload: I'd used `Buffer::get_mapped_range<S: RangeBounds<BufferAddress>>(&self, bounds: S)`, but both call sites are actually on `slice` (a `wgpu::BufferSlice`, from `let slice = buffer.slice(..)` a few lines earlier), and `BufferSlice::get_mapped_range()` — a distinct overload in a distinct `impl` block — takes **zero** arguments while still returning `Result<BufferView, MapRangeError>`. So `slice.get_mapped_range(..)` was passing an argument to a method that accepts none — a different compile error than round 1's, but still a compile error.

**Corrected fix**, in both `read_buffer_blocking` and `read_buffer_async` (`engine/src/gpu/mod.rs`):

```rust
let data = slice.get_mapped_range().expect("gpu buffer mapping failed");
```

(dropped the stray `(..)`, kept `.expect()` since the `Result` diagnosis itself was still correct both rounds.) Commit `83bd458`, 2 lines changed, no other file touched.

## Attempted the evaluator's suggestion: a minimal standalone `wgpu`-only scratch crate

Per the round-2 recommendation ("get an actual `cargo build` against this file specifically... rather than iterating further on citation-based verification alone"), I built the smallest possible test: a fresh scratch crate with `wgpu = "30.0.0"` as its *only* dependency, nothing else.

```toml
[package]
name = "wgpu-scratch"
version = "0.1.0"
edition = "2021"

[dependencies]
wgpu = "30.0.0"
```

Result: identical failure, now on a different transitive crate (`arrayvec`, not `allocator-api2` or `wgpu-types` as in earlier attempts against the full project):

```
error: failed to download from `https://static.crates.io/crates/arrayvec/0.7.8/download`
[56] Failure when receiving data from the peer (CONNECT tunnel failed, response 403)
```

This confirms what the RFI response already established structurally (this sandbox's own proxy `noProxy` config lists `index.crates.io` but not `static.crates.io`): the block is host-level and applies uniformly to every crate in `wgpu`'s dependency tree, regardless of project size or scope. A minimal scratch crate doesn't route around it — there is no way to get a real compiler in the loop for this dependency from inside this sandbox as currently configured. I'd already reached this conclusion via the RFI investigation, but wanted to actually run the suggested experiment rather than just cite the earlier finding, given the evaluator's own citation-based method had just been shown fallible twice in a row.

## Everything else — unchanged from round 2, all independently reconfirmed by the round-2 evaluator already

- The four other API-shape checks (single-argument `request_device`, `Result`-returning `request_adapter`, `Option`-wrapped `bind_group_layouts`, `immediate_size` not `push_constant_ranges`) are unaffected by either round's fix.
- Diff scope is still exactly the round-1 file set (`gpu/mod.rs` new; `context.rs`, `app.rs`, `lib.rs`, `Cargo.toml`, `Cargo.lock`, `app.js` wiring) plus these report files. No file under `operations/` touched.
- AC1 (native `cargo build`/`cargo test`) and AC3 (the `init_gpu()` → `context()` wiring test) remain unverifiable in this sandbox for the same structural reasons flagged in rounds 1 and 2 — network policy, and `app.rs`'s wasm32-only gating with no reachable wasm32 toolchain. Neither is new this round.

## On verification method going forward

Two rounds of citation-based (real-source-lookup) review each produced one precise, real, but incomplete correction — round 1 correctly said "this needs `Result` handling" but cited the wrong overload's exact signature; round 2 correctly caught that overload mismatch but I don't have a way to independently cross-check my round-3 fix against anything beyond the same method. I'm fairly confident in `slice.get_mapped_range().expect(...)` at this point (it's the exact shape both the reference code being restored *toward* and the evaluator's own `impl<'a> BufferSlice<'a>` quote agree on), but I want to be explicit that "fairly confident via citation" is exactly the standard that's now produced two rounds of near-misses — a real compiler pass is the only thing that would actually close this out with certainty, and neither of us has one available in-sandbox. If this fix is confirmed correct next round, I'd suggest not treating that as proof the citation method is reliable going forward — it's proof this specific fix matches every citation gathered so far, which is a narrower claim.
