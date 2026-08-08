# Evaluation: WebGPU operations Phase 1.3 — pointwise two-buffer ops

**Branch:** `claude/agent-setup-prep-oyj2ri` (not yet merged)
**Spec:** `SPECwebgpuoperations.md` (Phase 1.3), pattern from `SPECwebgpucomputebackend-1.md`, shape approved in Phase 0/1.1/1.2
**Report:** `.agents/communication/implementation_reports/webgpu/operations_Phase1_3_twobuffer_report.md`
**Commits:** `1833eb2` (`add.rs`), `e77d42a` (`screen.rs`), `12ea60a` (`subtract.rs`), `be15241` (`multiply.rs`), `828f5c0` (`mix.rs`), `b6ac44a` (`hue_key.rs`) — six independent, one file each. This closes out Phase 1 (1.1/1.2/1.3) once merged.

## 0. Build-environment — same restriction, re-confirmed

`index.crates.io`/`static.crates.io` still 403 in this session. Manual/static review only; **build/test acceptance criteria remain UNVERIFIED**, per `ENVIRONMENT_DIAGNOSTICS.md`, same as every prior phase.

## 1. Scope and structural conformance (all six)

- Each commit touches exactly one file. `gpu` module API usage matches the real signatures already verified across every prior phase — no new API surface. ✅
- No external struct-literal breakage for any of the six structs. ✅
- **Architectural improvement over Phase 0/1.1's pattern, confirmed:** every prior single-input operation's GPU-dispatch branch called `FloatImage::from_value` a second time (once for the GPU upload, once again for the CPU fallback on the same tick), noted as a minor accepted nit in both the `BLUR` and Phase 1.1 evaluations. This phase's two-buffer operations resolve both `FloatImage`s once, up front (needed regardless for the pre-existing dimension-match check), and pass clones into `dispatch_gpu` — eliminating that duplicate-resolve pattern entirely for all six operations here, not just avoiding it in one. Verified directly in each diff (`first_image.clone()`/`second_image.clone()`, or `source_image.clone()`/`reference_image.clone()` for `HUE_KEY`, passed to `dispatch_gpu`, never a second `from_value` call in the GPU branch). Worth noting as a genuine improvement, not just parity.

## 2. `ADD`, `SCREEN`, `SUBTRACT` — ✅ correct, identical shape

All three follow the exact same structure, differing only in per-channel formula:

- `ADD`: `foreground[idx] + background[idx]` — matches `add_pixels`/`add_single_pixel` exactly.
- `SCREEN`: `1.0 - (1.0 - a) * (1.0 - b)` — matches `screen_pixels`/`screen_single_pixel` exactly.
- `SUBTRACT`: `foreground[idx] - background[idx]` — matches `subtract_pixels`/`subtract_single_pixel` exactly.

All three: masked path (union of `Foreground`'s and `Background`'s boxes, the bbox work's established reasoning for non-zero-preserving-on-either-input operations) confirmed byte-for-byte unchanged, relocated into its own early-return branch ahead of GPU code. Fingerprints correctly key on both `Value`s via `value_ptr_eq`, no extra parameters needed (none of the three have tunable parameters). ✅

## 3. `MULTIPLY` — ✅ correct, and the masked-path scope decision (RFI Q1) is the right call

Confirmed against `dev`'s pre-existing `multiply.rs`: its masked path genuinely is unrestricted full-frame CPU compute (`Self::multiply_pixels(&first_image.pixels, &second_image.pixels)` called unconditionally, then `apply_mask` applied afterward) — `MULTIPLY` was never migrated to bbox-consumption, unlike `ADD`/`SCREEN`/`SUBTRACT`. This diff leaves that path completely untouched (still full-frame CPU, still unrestricted) and only attempts GPU dispatch when `mask.is_none()`, identical blanket-rule gating to every other operation. ✅

**RFI Q1 answer:** the conservative choice — not extending GPU dispatch to the masked path just because it happens to already be full-frame — is correct, for two reasons beyond just "consistent with the pattern":

1. `SPECwebgpuoperations.md`'s blanket rule is stated once, unconditionally, for every phase in the document: "GPU dispatch only ever replaces an operation's **unmasked** compute path." It isn't qualified by "unless the masked path happens to already be full-frame" — extending it here would be a deviation from an explicitly stated spec-wide rule, not a natural reading of it.
2. Bbox-migration (restricting a masked path's actual compute region) and GPU-acceleration (this spec's job) are two distinct, separately-scoped workstreams (`BBOX_CONVENTIONS.md` vs. `SPECwebgpuoperations.md`/`SPECwebgpucomputebackend-1.md`). GPU-accelerating `MULTIPLY`'s masked path now — before it's ever migrated to bbox-consumption — would mean that whenever `MULTIPLY` *does* eventually get bbox-migrated (separate future work), that migration would have to retrofit GPU dispatch onto an already-GPU-accelerated masked path instead of a plain CPU one every other operation's bbox migration started from, entangling two workstreams that are cleaner kept apart. Correct to treat "already full-frame" as an unrelated fact about `MULTIPLY`'s current state, not license to expand this phase's scope.

## 4. `MIX` — ✅ correct

No `MASK` input (confirmed via `metadata()`), so GPU dispatch is unconditional, same shape as `RGB_TO_HSV`/`CHECKERBOARD`. Shader (`foreground*(1-amount)+background*amount`) matches `mix_pixels`'s `source_a[channel]*(1.0-amount)+source_b[channel]*amount` exactly. `AMOUNT` bit-packed via the same `f64→f32→bits` chain as every other scalar parameter in this codebase (`BLUR`'s radius, `CHROMAKEY`'s threshold, etc.), and correctly included in the fingerprint alongside both `Value`s. ✅

## 5. `HUE_KEY` — ✅ correct, `%` safety reasoning (RFI Q2) confirmed sound

- `target_hue` computed once, CPU-side, via `self.hue_color.to_hsv()` — same already-tested function `RGB_TO_HSV` calls — and passed down as a single uniform float, correctly avoiding re-porting the full RGB→HSV conversion into WGSL a second time for a value `HUE_KEY` only ever needs as a scalar. Confirmed this is computed once, before the masked/unmasked branch split, and used consistently for both. ✅
- `hue_distance` ported to WGSL (`fn hue_distance(a: f32, b: f32) -> f32`) matches the CPU version exactly: `abs(a-b) % 360.0` then `min(diff, 360.0-diff) / 180.0`. ✅
- Masked path (`SOURCE`'s box alone, no growth needed, `REFERENCE`'s box deliberately uninvolved — per the file's own pre-existing doc comment) confirmed byte-for-byte unchanged. ✅
- `select(source[idx+3u], 0.0, distance <= threshold)` reads correctly (`0.0` when keyed out, source alpha otherwise) and matches CPU's `if distance <= threshold {0.0} else {src[3]}` — both `select()` arguments are already-computed plain values, no discarded-branch risk (same safe category as `CHROMAKEY`'s identical shape). ✅

**RFI Q2 answer (the `%` safety argument):** sound. Verified two things directly rather than accepting the reasoning by assertion:

1. **WGSL's `%` for floats is truncated remainder** (`x - y * trunc(x/y)`), the same semantics as Rust's `%` for `f64`/`f32` — confirmed this is the relevant operator behavior, not something else. Truncated and Euclidean modulo only disagree when the *dividend* is negative (for a positive divisor, which `360.0` always is here); for a non-negative dividend they're identical by construction — Euclidean remainder is defined to always return a value in `[0, divisor)`, and truncated remainder already does that for non-negative dividends since it doesn't need to "correct" anything toward positive.
2. **The dividend genuinely is always non-negative here**, unconditionally: `hue_distance`'s dividend is `abs(a - b)`, and `abs()` produces a non-negative result *regardless of the sign of `a` or `b` themselves* — this holds even in a hypothetical case where `reference_hue` or `target_hue` were themselves negative (e.g., an out-of-gamut `REFERENCE` feeding `RGB_TO_HSV`, or a future scenario), since the `abs()` wrapping happens before the `%`, not after. This is a stronger, more general guarantee than "the inputs happen to be non-negative in practice" — it's structurally guaranteed by the formula's shape, independent of what `a`/`b` are.

This is a materially different (and correctly identified as different) situation from `RGB_TO_HSV`'s Phase 1.1 case (RFC-003): there, the divergence wasn't about `%`/`rem_euclid` at all — the bug was an unrelated guard-condition change (`max_c > 0.0` vs. `max_c != 0.0`). `HUE_KEY`'s reasoning for skipping the floor-mod emulation is independently correct on its own terms, not just "probably fine by analogy."

## 6. Recommendation

**✅ Approve all six: `ADD`, `SCREEN`, `SUBTRACT`, `MULTIPLY`, `MIX`, `HUE_KEY`.** No blocking or major defects in any of them. No RFC needed. Both phase-specific judgment calls (`MULTIPLY`'s masked-path scope, `HUE_KEY`'s `%` safety) are correct, verified independently rather than accepted by analogy to prior phases.

This closes out Phase 1 (1.1, 1.2, 1.3) — Phase 2 (`RESIZE`, `MOVE`) can proceed once this lands.
