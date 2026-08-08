# RFC-003: `RGB_TO_HSV`'s GPU saturation guard diverges from CPU for negative `max_c`

**Status:** Blocking merge of `engine/src/operations/transform/rgb_to_hsv.rs` (commit `4041d61` on `claude/agent-setup-prep-oyj2ri`) only. `CLAMP`, `INVERT`, `SHUFFLE`, `CHROMAKEY` (the other four Phase 1.1 commits) are unaffected and independently approved — see `evaluation_webgpu_operations_phase1_1_pointwise.md`.
**Type:** RFC, not a new specification — this is a correction against `SPECwebgpuoperations.md`'s Phase 1.1 acceptance criteria (GPU output must match CPU), not a design change.

## Finding

`RGB_TO_HSV`'s WGSL shader (`RGB_TO_HSV_SHADER` in `rgb_to_hsv.rs`) computes saturation (`s`) with a different guard condition than the existing, already-tested CPU reference (`Color::to_hsv()`, `engine/src/graphics/color.rs`):

```rust
// CPU (Color::to_hsv, unchanged, already tested)
let s = if max == 0.0 { 0.0 } else { delta / max };
```

```wgsl
// GPU (rgb_to_hsv.rs, this commit)
var s: f32 = 0.0;
if (max_c > 0.0) {
    s = delta / max_c;
}
```

`max == 0.0` (CPU) and `max_c > 0.0` (GPU) are **not** the same condition. CPU computes `s` for every `max != 0.0`, including negative `max`. GPU only computes `s` when `max_c` is strictly positive, silently leaving `s = 0.0` for any `max_c <= 0.0` — which includes every negative case, not just the zero case both branches agree on.

This matters because `RGB_TO_HSV` accepts a general `FloatImage` `SOURCE` with no clamping enforced anywhere upstream in the graph, and this codebase explicitly allows unclamped, out-of-gamut pixel data by design — `ADD`, `SUBTRACT`, and `SCREEN` are all documented as deliberately unclamped (see their own doc comments and `CLAMP`'s own "the one place an out-of-gamut value actually [gets clamped]" framing). A pixel with all-negative RGB (e.g. downstream of a `SUBTRACT` with no intervening `CLAMP`) reaching `RGB_TO_HSV` has a negative `max_c`, and the two implementations genuinely disagree:

```
r = -0.2, g = -0.5, b = -0.8
max_c = -0.2, min_c = -0.8, delta = 0.6

CPU:  max != 0.0  →  s = delta / max = 0.6 / -0.2 = -3.0
GPU:  max_c > 0.0 is false  →  s = 0.0
```

A difference of `3.0` is far outside the pattern spec's `1e-4` GPU-vs-CPU numerical tolerance — this is a real behavioral divergence, not float rounding noise. It is silent: no error is raised, `is_out_of_gamut`-style warnings aren't triggered by saturation specifically, and the wrong saturation value would flow straight into whatever reads `RGB_TO_HSV`'s output (per its own doc comment, primarily `HUE_KEY`), producing visibly incorrect keying results whenever the GPU path is active for a graph with unclamped upstream content, with no equivalent symptom on the CPU-only path.

## Why this wasn't caught by the existing GPU-vs-CPU test

`gpu_rgb_to_hsv_matches_cpu_within_tolerance_once_warmed_up` only exercises pixels built from `u8` source values (`0..255`), which convert to `FloatImage` pixels in `0.0..=1.0` — always non-negative. The test can't reach the diverging branch because its inputs never produce a negative `max_c`. This isn't a flaw in the test's own design goal (numerical-tolerance comparison against realistic display-range input) — it just doesn't cover the out-of-gamut case that's the actual root cause here, which is a separate, deliberate capability of this codebase, not an unusual/synthetic edge case.

## Required change

In `RGB_TO_HSV_SHADER`, change the guard from:

```wgsl
if (max_c > 0.0) {
    s = delta / max_c;
}
```

to:

```wgsl
if (max_c != 0.0) {
    s = delta / max_c;
}
```

This is a one-line fix, doesn't reintroduce the `select()`-vs-`if` question (still a real `if`, still short-circuits the division, no discarded-branch computation either way — a `!=` comparison isn't any less safe than a `>` one), and restores exact parity with the CPU guard.

## Suggested additional test

A regression test covering exactly this case would prevent silent recurrence — something like:

```rust
#[test]
fn gpu_rgb_to_hsv_matches_cpu_for_out_of_gamut_negative_channels() {
    // Same GPU-vs-CPU comparison shape as the existing tolerance test,
    // but with a FloatImage built directly (bypassing U8Image, which
    // can't represent negative values) so max_c goes negative - the
    // case the u8-sourced test above structurally cannot reach.
    // ... construct a FloatImage with e.g. r=-0.2, g=-0.5, b=-0.8 ...
    // ... run both RgbToHsv::convert_pixels (CPU) and the GPU dispatch ...
    // ... assert saturation channels match within tolerance ...
}
```

Not requiring an exact test shape — the point is coverage of a negative-`max_c` input specifically, since the existing test structurally cannot exercise it.

## Acceptance condition

1. The guard in `RGB_TO_HSV_SHADER` matches `Color::to_hsv()`'s `max == 0.0`/`max != 0.0` condition exactly (not `max_c > 0.0`).
2. A test demonstrating GPU/CPU parity for a negative-`max_c` input (out-of-gamut `FloatImage`, not `U8Image`-sourced) passes.
3. No other change needed — the rest of `RGB_TO_HSV`'s port (hue formula, `rem_euclid` emulation, alpha passthrough, dispatch/fingerprint/caching structure) is correct as reviewed and does not need to change.
