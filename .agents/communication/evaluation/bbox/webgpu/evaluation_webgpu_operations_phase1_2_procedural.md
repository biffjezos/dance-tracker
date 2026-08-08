# Evaluation: WebGPU operations Phase 1.2 — procedural zero-buffer ops

**Branch:** `claude/agent-setup-prep-oyj2ri` (not yet merged)
**Spec:** `SPECwebgpuoperations.md` (Phase 1.2), pattern from `SPECwebgpucomputebackend-1.md`, shape approved in Phase 0/1.1
**Report:** `.agents/communication/implementation_reports/webgpu/operations_Phase1_2_procedural_report.md`
**Commits:** `7511297` (`checkerboard.rs`), `55af09e` (`ring.rs`) — two independent, one file each.

## 0. Build-environment — same restriction, re-confirmed

`index.crates.io`/`static.crates.io` still 403 in this session. Manual/static review only; **build/test acceptance criteria remain UNVERIFIED**, per `ENVIRONMENT_DIAGNOSTICS.md`, same as every prior phase.

## 1. Scope and structural conformance

- Each commit touches exactly one file. Neither operation has a `MASK` input (confirmed via full-file `grep` for `Mask`/`mask` on both — no matches) — correctly, per the spec's own note, "vacuously satisfied," no masked-path branch needed or added. ✅
- No external `Op { field: value }` struct literal anywhere in the codebase for either struct that the new private fields would have broken. ✅
- `gpu` module API usage matches the real signatures already verified in Phase 0/1.1's evaluations — no new API surface. ✅

## 2. `CHECKERBOARD` — ✅ correct

- Shader's `checker`/`select()` logic matches `generate()`'s `((x/tile)+(y/tile))%2==0` exactly, and `select(color_b, color_a, checker)` returns `color_a` when `checker` is true — matching CPU's `if checker { a } else { b }`. Both `select()` arguments are already-computed plain values, no discarded-branch risk (correctly distinguished from the `RGB_TO_HSV`/RFC-003 situation, as the report notes). ✅
- `tile = self.size.max(1.0) as u32` computed identically Rust-side for both the CPU path and the GPU fingerprint/params — same zero-guard, so the shader's `id.x / tile` can't divide by zero. ✅
- Params packing (`[u32; 12]` → `array<vec4<u32>, 3>`, `width/height/tile` then `color_a`'s 4 channels then `color_b`'s 4 channels) traced index-by-index against the shader's `params[0].w`/`params[1].{x,y,z,w}`/`params[2].{x,y,z}` reads — every index lines up correctly. ✅
- **Quantization rule (RFI Q1, `CHECKERBOARD` half):** correct choice. `Checkerboard::generate()`'s CPU path calls `self.color_a.to_rgba_u8()`/`self.color_b.to_rgba_u8()` — confirmed in `color.rs`, `to_rgba_u8` is `(self.c.clamp(0.0, 1.0) * 255.0) as u8`, a **truncating** cast (no `.round()`), unlike `to_image_clamped`'s rounding one. The GPU path's post-readback quantization (`(c.clamp(0.0, 1.0) * 255.0) as u8`) is the identical truncating formula. Since `CHECKERBOARD` never blends — every output pixel is either `color_a` or `color_b` verbatim, never an average — applying the truncating cast per-pixel-after-readback vs. once-per-color-then-copied are mathematically identical operations on identical float bits. Bit-exact match confirmed, not just "close enough." ✅
- Fingerprint keyed on `(width, height, tile, color_a, color_b)` via real equality (no `Value` to compare — see §4). Uses the *derived* `tile`, not raw `size`, correctly — since the shader's actual output only depends on `tile`, keying on the lossy-quantized value is the right choice (avoids needless cache invalidation on a `size` change that doesn't cross a tile boundary), not a missed field.

## 3. `RING` — ✅ correct, one non-blocking precision observation

Reviewed the shader loop against `generate()` line-by-line:

- Loop bound: CPU `1..=self.count`, GPU `for (var ring_number = 1u; ring_number <= count; ...)` — same inclusive range, same starting point. ✅
- `ring_radius = radius - (ring_number - 1) * spacing` matches exactly (f32 vs. f64 narrowing, same expected precision gap as every other float parameter in this phase). `if ring_radius < 0.0 { continue; }` matches CPU's identical guard. ✅
- Distance: `cx/cy = width/height / 2`, `dx/dy = coord + 0.5 - center`, `dist = sqrt(dx²+dy²)` — same formula shape as `generate()`, f32 (GPU) vs. f64 (CPU) precision, the standard expected gap. ✅
- Hit test (`abs(dist - ring_radius) <= half_thickness`) and "first match wins, `break`" both match `generate()` exactly, including that a pixel matching no ring keeps the initial `vec4<f32>(0,0,0,0)` (GPU) / untouched `0u8` buffer (CPU) — both default to fully transparent black. ✅
- **Runtime-sized colors storage buffer (RFI Q2 — the specific concern raised):** verified the WGSL/wgpu shape directly. `array<vec4<f32>>` as a bare top-level storage binding (not wrapped in a struct) is valid WGSL for a runtime-sized array. The Rust-side upload (`fingerprint.colors.iter().flat_map(|c| [c.r,c.g,c.b,c.a]).collect()`, a flat `Vec<f32>`) produces a byte layout that's bit-identical to what WGSL expects for `array<vec4<f32>>` in the storage address space: `vec4<f32>`'s storage-class alignment equals its size (16 bytes, 4×4-byte floats, no internal padding), so each `colors[i]` reads exactly the 4 floats at flat offset `4i..4i+4` — precisely how the Rust side laid them out. `BufferBindingType::Storage { read_only: true }` with `min_binding_size: None` is correct for a runtime-sized array bound via `as_entire_binding()` — wgpu infers the actual usable length from the bound buffer's real size at bind-group-creation time; no static `min_binding_size` is needed since the shader doesn't call `arrayLength()` itself (it trusts the uniform `count`, which the Rust side keeps in sync with `colors.len()` via `set_count`, as the report notes). This matches the exact mechanism `gpu/mod.rs`'s own `DOUBLE_SHADER` test already establishes works (per the foundation evaluation). **Correctly shaped.** ✅
- Params packing (`[u32; 8]` → `array<vec4<u32>, 2>`: `width/height/count/radius_bits` then `spacing_bits/thickness_bits`) traced against `params[0].w`/`params[1].{x,y}` — indices line up. ✅
- **Quantization rule (RFI Q1, `RING` half):** same as `CHECKERBOARD` — `generate()` calls `self.colors[ring_number-1].to_rgba_u8()` (truncating), GPU path's post-readback quantization uses the identical truncating formula. Correct, for the same "never blends, only ever copies a color verbatim" reasoning. ✅
- Fingerprint keyed on `(width, height, count, radius_bits, spacing_bits, thickness_bits, colors)` — every value `generate()` actually depends on is present; nothing missing. `colors: Vec<Color>` compared by real (derived, presumably `#[derive(PartialEq)]` on `Color`, already relied on elsewhere e.g. `ChromaKeyFingerprint`) equality, appropriate since the whole vector — not just a hash or length — determines output. ✅

**Non-blocking observation, same category as `CHROMAKEY`'s (Phase 1.1 evaluation):** the ring hit-test `abs(dist - ring_radius) <= half_thickness` is, like `CHROMAKEY`'s keying, a hard threshold comparison rather than a continuous output. A pixel whose true distance sits extremely close to a ring boundary could in principle land on opposite sides of `<=` under `f32` (GPU) vs. `f64` (CPU) precision, flipping "on the ring" vs. "not," rather than producing a small color-value wobble. Inherent to mixed-precision thresholding, not a coding defect, not something either test (this one or `CHROMAKEY`'s) is designed to catch. Not requesting any change.

## 4. Answers to the RFI's specific questions

**Q1 — Quantization rule, right choice for both operations?** Yes, confirmed for both — see §2/§3. Both correctly use `to_rgba_u8`'s truncating cast, not `to_image_clamped`'s rounding one, matching what each operation's own CPU path actually calls.

**Q2 — Is `RING`'s colors storage buffer bind group layout correctly shaped for a runtime-sized array?** Yes, confirmed by direct WGSL/wgpu semantics, not just accepted by analogy to `gpu/mod.rs`'s test — see §3's detailed trace.

**Q3 — Is the no-`Value` fingerprint shape (keyed on `width`/`height`/parameters via real equality) right for a no-input operation?** Yes, checked rather than accepted by analogy: for both operations, every field the shader/CPU path's actual output depends on is present in the fingerprint (traced against `generate()`'s own dependencies field-by-field for both), and real equality is the only sensible comparison when there's no wired `Value` to compare by pointer identity — `value_ptr_eq` exists specifically because pixel buffers are too expensive to compare by value, but scalar parameters and a small `Vec<Color>` are cheap enough that real equality is both correct and appropriately efficient here.

## 5. Recommendation

**✅ Approve both `CHECKERBOARD` and `RING`.** No blocking or major defects in either. `RING`'s precision-boundary note (§3) is informational only, same category as the already-accepted `CHROMAKEY` observation from Phase 1.1, not blocking.
