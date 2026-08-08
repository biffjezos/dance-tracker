# Evaluation: WebGPU operations Phase 1.1 — pointwise single-buffer ops

**Branch:** `claude/agent-setup-prep-oyj2ri` (not yet merged)
**Spec:** `SPECwebgpuoperations.md` (Phase 1.1), pattern from `SPECwebgpucomputebackend-1.md`, approved shape from Phase 0 (`BLUR`, `evaluation_webgpu_operations_phase0_blur.md`)
**Report:** `.agents/communication/implementation_reports/webgpu/operations_Phase1_1_pointwise_report.md`
**Commits:** `24e5409` (`clamp.rs`), `64fc23f` (`invert.rs`), `4041d61` (`rgb_to_hsv.rs`), `d6ae6f7` (`shuffle.rs`), `b0bc8cb` (`chromakey.rs`) — five independent, one file each.

## 0. Build-environment — same restriction, re-confirmed

`index.crates.io`/`static.crates.io` both still 403 in this session — identical to Phase 0 and `ADD`'s evaluations, same tracked restriction (`notification_cargo_registry_index_blocked.md`). Manual/static review only; **build/test acceptance criteria remain UNVERIFIED, not passing**, per `ENVIRONMENT_DIAGNOSTICS.md`.

## 1. Scope, pattern-shape, and blanket-rule compliance (all five)

- Each commit touches exactly one file, matching the report's claim. ✅
- Every masked path (`INVERT`, `SHUFFLE`, `CHROMAKEY` — `CLAMP` and `RGB_TO_HSV` have no `MASK` input, confirmed via `metadata()`/full-file `grep` for `Mask` on each) is byte-for-byte unchanged, relocated into its own early-return branch ahead of GPU code, same shape as `BLUR`. Checked each diff directly against its pre-existing masked-path logic — no line changed beyond re-indentation/relocation. ✅
- `Rc<RefCell<...>>`/`gpu_pipeline`/`pending`/`last_gpu_result`/`is_live()`/target-conditional-readback structure is a mechanical, faithful copy of `Blur`'s already-approved shape in every one of the five files — checked each `dispatch_gpu` against `blur.rs`'s own for structural drift; none found. ✅
- No operation outside these five had a bare `Op { field: value }` struct literal anywhere in the codebase that the new private fields would have broken (`git grep` per struct name, only the struct/impl declarations themselves match) — consistent with the report's claim, and consistent with why `CLAMP`/`INVERT`/`RGB_TO_HSV`/`SHUFFLE`/`CHROMAKEY` needed no test struct-literal fix the way `BLUR` did. ✅
- `gpu` module API usage (`create_shader`, `create_compute_pipeline`, `upload`, `create_buffer`, `create_bind_group`, `dispatch`, `copy_buffer_to_buffer`, `read_buffer_blocking`/`read_buffer_async`) matches the real signatures already checked against source in the foundation and `BLUR` evaluations — no new API surface introduced beyond what those two already verified. ✅

## 2. Per-operation review

### `CLAMP` — ✅ correct, answers RFI Q2 directly

- No `MASK` input — GPU dispatch unconditional when available, matches `metadata()`. ✅
- Shader computes `clamp(input, min_v, max_v)` per channel only — matches `to_image_clamped`'s own `c.clamp(min, max)` step exactly, leaving quantization out of the shader (correctly, since `gpu/mod.rs`'s readback only returns `Vec<f32>`).
- **Quantization parity (the RFI question):** `to_image_clamped` is `(c.clamp(min, max) * 255.0).round() as u8` — Rust's `as u8` float-to-int cast **saturates** (post-1.45 semantics), not wraps; the repo's own test (`to_image_clamped_respects_a_custom_range`) explicitly documents this (`1.5 clamped to 1.5, *255 saturates to 255`). The GPU path's CPU-side quantization is `(c * 255.0).round() as u8` applied to the already-GPU-clamped value — same formula, same saturating cast, just with the clamp done GPU-side instead of CPU-side first. **Confirmed bit-for-bit equivalent**, including the saturation edge case for a custom range like `max = 1.5`. No rounding/saturation edge case missed. ✅
- `min_bits`/`max_bits` bit-packing: `f64::to_bits()` (fingerprint, `u64`) → `f64::from_bits(...) as f32` → `.to_bits()` (`u32`, shader param) → `bitcast<f32>(...)` in WGSL. Traced the full round trip; correct, no truncation beyond the intended f64→f32 narrowing (same narrowing every other float parameter in this phase/`BLUR` already accepts).

### `INVERT` — ✅ correct

- Masked path (`MASK`'s own box alone, not intersected with `SOURCE`'s — deliberately, since `INVERT` isn't zero-preserving) confirmed unchanged.
- Shader: `1.0 - pixel` on all four channels, matching `invert_pixels`'s `1.0 - channel` uniformly including alpha (matches the file's own doc comment convention). ✅
- No trivial-identity short-circuit needed or added, correctly noted as intentional (no radius-like no-op case exists for `INVERT`). ✅

### `RGB_TO_HSV` — ⚠️ one defect found, see RFC-003

Reviewed the WGSL port line-by-line against `Color::to_hsv()`. Most of it is correct and carefully done:

- `rem_euclid` emulation (`raw - 6.0 * floor(raw / 6.0)`) is the exact standard floor-mod identity Rust's `f64::rem_euclid` implements. Verified numerically against the existing `to_hsv_hue_never_goes_negative` test's own worked case (magenta, `r=1,g=0,b=1`): `max=r`, `delta=1`, `raw=(g-b)/delta=-1`, `rem_euclid: -1 - 6*floor(-1/6) = -1 - 6*(-1) = 5`, `h = 60*5 = 300` — matches the test's asserted `300.0` exactly. ✅
- Branch order (`max_c == r` → `== g` → else `b`) matches the CPU `if max==r elif max==g else` exactly, and floating-point `max()` is associative/commutative for finite non-NaN values, so `max(r, max(g,b))` (WGSL) and `r.max(g).max(b)` (CPU) are bit-identical — the tie-breaking order can't drift between the two. ✅
- `delta != 0.0` guard matches CPU's `delta == 0.0` (inverse of the same condition) exactly. ✅
- Hue formula for the `g`/`b`-max branches (`+2.0`/`+4.0`, no `rem_euclid` needed there) matches CPU exactly — correctly recognized these branches don't need the wraparound emulation. ✅
- Alpha passthrough (`a = input[idx+3]`) matches CPU's `source[3]` passthrough exactly. ✅

**The defect:** the saturation guard changed semantics, not just mechanism. CPU: `s = if max == 0.0 { 0.0 } else { delta / max }` — computes `s` for **any** `max != 0.0`, including negative `max`. GPU: `if (max_c > 0.0) { s = delta / max_c; }` — only computes `s` for **strictly positive** `max_c`, leaving `s = 0.0` for `max_c <= 0.0` (zero *and* negative both fall through to the default).

This isn't equivalent to the (correct, and separately justified) `if`-vs-`select()` choice — it's a different guard condition. `RGB_TO_HSV` accepts a general `FloatImage` `SOURCE` with no clamping enforced anywhere upstream, and this codebase explicitly supports out-of-gamut/unclamped pixel data by design (`ADD`/`SUBTRACT`/`SCREEN` are all documented as deliberately unclamped). A pixel reaching `RGB_TO_HSV` with all-negative RGB (e.g., downstream of a `SUBTRACT` producing a negative result, with no intervening `CLAMP`) has a negative `max_c`, and the two paths genuinely diverge:

```
r=-0.2, g=-0.5, b=-0.8  →  max_c=-0.2, min_c=-0.8, delta=0.6
CPU: max != 0.0  → s = delta/max = 0.6 / -0.2 = -3.0
GPU: max_c > 0.0 is false → s = 0.0
```

A difference of `3.0` is nowhere near the `1e-4` numerical tolerance the pattern spec allows for GPU-vs-CPU float slop — this is a real behavioral divergence, not rounding noise, and it's silent (no error, just visibly wrong saturation data feeding whatever reads this operation's output, e.g. `HUE_KEY`). See RFC-003 for the required fix (`max_c > 0.0` → `max_c != 0.0`).

### `SHUFFLE` — ✅ correct

- Masked path (`SOURCE`'s box intersected with `MASK`'s) confirmed unchanged.
- `array<vec4<u32>, 2>` params layout traced byte-for-byte: Rust `[u32; 8]` is 32 bytes, elements 0–3 and 4–7 packed contiguously with no gaps; WGSL's two `vec4<u32>` are likewise 16 bytes each with no internal padding (element size == alignment == stride, satisfying the "array stride must be a multiple of 16 bytes" uniform rule trivially, as the code comment claims). `params[0].{x,y,z,w}` → Rust indices `[0,1,2,3]` (`width, height, red_sel, green_sel`), `params[1].{x,y}` → indices `[4,5]` (`blue_sel, alpha_sel`) — matches the upload array's construction order exactly. ✅
- `channel_value()`'s `sel` 0–3 mapping (R/G/B/A) matches `to_gpu_selector()`'s own mapping and the CPU `channel_value`'s `pixel[0..3]` field order exactly; `Off`'s CPU behavior (`T::default()`, i.e. `0.0` for `f32`) matches the shader's `sel==4` fallthrough (`return 0.0;`). ✅

### `CHROMAKEY` — ✅ correct, one non-blocking observation

- Masked path (`SOURCE`'s box intersected with `MASK`'s, same as `SHUFFLE`) confirmed unchanged.
- Distance formula (`sqrt(dr²+dg²+db²)/sqrt(3)`) and the `<= threshold → alpha 0` rule both match `key_pixels`/`key_single_pixel` exactly. `select(a, 0.0, distance <= threshold)` reads correctly per `select`'s `(false_value, true_value, cond)` order — `0.0` when keyed out, original alpha `a` otherwise, matching CPU's `if distance <= threshold { 0.0 } else { source[3] }`. Both `select()` arguments here are already-computed plain values (no risky discarded-branch computation), correctly distinguished from `RGB_TO_HSV`'s situation in the report. ✅
- Params layout (`array<vec4<u32>, 2>`, `key_r`/`key_g`/`key_b`/`threshold` bit-packed via `to_bits()`/`bitcast<f32>`) traced the same way as `SHUFFLE`'s — indices line up correctly. ✅
- **Non-blocking observation:** CPU computes the distance in `f64` (`as f64` promotion mid-calculation, even though the source pixels are `f32`), GPU computes it in `f32` throughout. For a continuous output (like `BLUR`'s averaged pixel) this kind of precision gap is exactly what the pattern spec's numerical-tolerance allowance is for. `CHROMAKEY`'s output, though, is a **hard step function** of `distance <= threshold` — a pixel whose true distance sits extremely close to `threshold` could in principle round to opposite sides of the comparison under `f32` vs `f64`, flipping fully-keyed vs. fully-opaque rather than producing a small numeric wobble. This is a narrow, inherent property of thresholding under mixed float precision, not a coding defect, and not something the existing test (deliberately using colors either very close to or clearly far from the key color) would catch either way. Worth being aware of if a future bug report describes a chroma-keyed edge flickering between GPU/CPU-cached ticks; not worth blocking this phase on.

## 3. Answers to the RFI's two specific questions

**Q1 — Is `if` vs. `select()` the right line to draw for `RGB_TO_HSV`'s zero-guards, or is WGSL float divide-by-zero safe enough that `select()` would have been fine?**

WGSL floating-point division follows IEEE-754 semantics: `x / 0.0` produces `±Infinity`, `0.0 / 0.0` produces `NaN` — well-defined, not a trap, not UB. So a `select()` computing `delta / max_c` unconditionally in its discarded branch (mirroring `BLUR`'s safe-discarded-underflow precedent) would **not** have caused a crash or UB either — it would just compute `Inf`/`NaN` and then discard it. In that narrow sense, the caution wasn't strictly *required* for safety, and the same general category as `BLUR`'s `select()` usage does apply here after all.

That said: using real `if` branches here was still the right call, independent of the safety question — it's clearer to read, costs nothing (these are scalar, not per-invocation-parallel hot paths where branch divergence would matter much at this scale), and, as it turns out, is *exactly* where the actual introduced bug lives, unrelated to the `if`-vs-`select()` choice itself (see §2's `RGB_TO_HSV` finding — the bug is the guard *condition* being `max_c > 0.0` instead of `max_c != 0.0`, which would have been just as wrong written as a `select()`). So: not a required safety distinction, but a reasonable one to keep, and orthogonal to the actual defect found.

**Q2 — Does `CLAMP`'s CPU-side quantization correctly reproduce `to_image_clamped`'s exact behavior?**

Yes — see §2's `CLAMP` section. Confirmed bit-for-bit equivalent, including the saturating-cast edge case for custom ranges beyond `0.0..1.0`.

## 4. Bundling question

Bundling all five into one RFI was a reasonable call given four of the five needed no correction — splitting into five round-trips wouldn't have caught the `RGB_TO_HSV` issue any faster, and this review was able to fully evaluate all five in one pass since the differences between them are narrow and independently checkable, exactly as the report predicted. No objection to the same approach for Phase 1.2/1.3.

## 5. Recommendation

**✅ Approve `CLAMP`, `INVERT`, `SHUFFLE`, `CHROMAKEY`** — no blocking or major defects in any of the four; `CHROMAKEY`'s precision-boundary note (§2) is informational, not blocking.

**⚠️ `RGB_TO_HSV` needs a correction before merge** — see RFC-003 (`.agents/communication/rfc/RFC003rgbtohsvgpusaturationguard.md`) for the specific required change. The other four commits are independent and unaffected; per the report's own framing, this doesn't need to block them.
