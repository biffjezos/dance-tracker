use std::cell::Cell;

use crate::compositor::{bbox::Rect, Context, OperationError, Value};
use super::float_image::FloatImage;

thread_local! {
    /// How many pixels the most recent `compute_within_bbox` call actually
    /// computed - read (and cleared) by `RenderExecutor::execute_profiled`
    /// immediately after each node's own `execute()` returns, so
    /// `ProfileEntry::pixels_computed` can report real work done without
    /// threading a counter through every `execute()` signature. Reset
    /// before each profiled `execute()` call, so a node that doesn't call
    /// `compute_within_bbox` at all correctly reports `None`, not a stale
    /// count left over from a previous node.
    static LAST_PIXELS_COMPUTED: Cell<Option<u32>> = const { Cell::new(None) };
}

/// Clears the pixel-compute counter - call immediately before a profiled
/// `execute()` so a leftover count from an earlier node's call can never
/// be misattributed to one that never calls `compute_within_bbox`.
pub fn reset_pixels_computed() {
    LAST_PIXELS_COMPUTED.with(|cell| cell.set(None));
}

/// Reads (and clears) the pixel-compute count left by the most recent
/// `compute_within_bbox` call, if any.
pub fn take_pixels_computed() -> Option<u32> {
    LAST_PIXELS_COMPUTED.with(|cell| cell.take())
}

#[derive(Debug)]
pub struct Mask {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Resolve an operation's wired MASK input to float RGBA + dimensions -
/// delegates entirely to `FloatImage::from_value`, so a mask can be wired
/// from a bounded U8Image/Frame/Video or an unbounded FloatImage exactly
/// like any other pixel-bearing input, with no special-casing here.
pub fn resolve_pixels(value: &Value, ctx: &Context) -> Result<(Vec<f32>, u32, u32), OperationError> {
    let float_image = FloatImage::from_value(value, ctx)?;
    Ok((float_image.pixels, float_image.width, float_image.height))
}

/// Blend `processed` toward `original` per pixel, weighted by the wired
/// mask's own alpha channel, clamped to 0.0..1.0 - a blend weight outside
/// that range has no sensible meaning the way an out-of-gamut colour/light
/// value does, so this is a deliberate, narrow exception to "no implicit
/// clamping" (0 = fully original/identity, 1 = fully processed). Neither
/// `original` nor `processed` is clamped, though: blending partway toward
/// an out-of-gamut `processed` value can correctly still be out of gamut
/// (see graphics::FloatImage) - only the weight itself is bounded.
///
/// The one shared mechanism behind every operation's optional MASK input,
/// so the blend itself is implemented exactly once rather than once per
/// operation. `mask` is `(pixels, width, height)`; passing `None`
/// (nothing wired) returns `processed` untouched.
pub fn apply_mask(
    original: &[f32],
    processed: Vec<f32>,
    mask: Option<&(Vec<f32>, u32, u32)>,
    width: u32,
    height: u32,
) -> Result<Vec<f32>, OperationError> {
    let Some((mask_pixels, mask_width, mask_height)) = mask else {
        return Ok(processed);
    };

    if *mask_width != width || *mask_height != height {
        return Err(OperationError::InvalidInputType(format!(
            "MASK is {}x{}, but the node it's masking is {}x{}",
            mask_width, mask_height, width, height
        )));
    }

    let mut out = vec![0f32; processed.len()];
    for i in (0..processed.len()).step_by(4) {
        let weight = mask_pixels[i + 3].clamp(0.0, 1.0);
        for c in 0..4 {
            let o = original[i + c];
            let p = processed[i + c];
            out[i + c] = o * (1.0 - weight) + p * weight;
        }
    }

    Ok(out)
}

/// Only invokes `compute` for pixels inside `work_area`; every pixel
/// outside it is copied directly from `original` - the one place
/// "restrict to a bbox, else pass through" is implemented, reused by
/// every masked operation instead of each hand-rolling its own
/// restricted loop. See BBOX_CONVENTIONS.md's Phase 3.
///
/// `work_area` is clamped to `[0,width) x [0,height)` here (not assumed
/// already-clamped by the caller) - a reported box from an upstream
/// operation is expected to already be frame-bounded (Phase 1/2 both
/// intersect against `Rect::full` before returning), but clamping here
/// too costs nothing and removes any risk of an out-of-bounds index if
/// that assumption is ever violated.
///
/// Records how many pixels `compute` was actually called for via
/// `LAST_PIXELS_COMPUTED` (see `take_pixels_computed`) - the mechanism
/// `RenderExecutor::execute_profiled` reads to populate
/// `ProfileEntry::pixels_computed`, proving real work was skipped rather
/// than just asserting it.
pub fn compute_within_bbox(
    width: u32,
    height: u32,
    work_area: Rect,
    original: &[f32],
    compute: impl Fn(u32, u32) -> [f32; 4],
) -> Vec<f32> {
    let mut output = original.to_vec();
    let mut computed = 0u32;

    if !work_area.is_empty() {
        let x0 = work_area.x0.max(0) as u32;
        let y0 = work_area.y0.max(0) as u32;
        let x1 = (work_area.x1.max(0) as u32).min(width);
        let y1 = (work_area.y1.max(0) as u32).min(height);

        for y in y0..y1 {
            for x in x0..x1 {
                let idx = ((y * width + x) * 4) as usize;
                let pixel = compute(x, y);
                output[idx..idx + 4].copy_from_slice(&pixel);
                computed += 1;
            }
        }
    }

    LAST_PIXELS_COMPUTED.with(|cell| cell.set(Some(computed)));

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_mask_returns_processed_unchanged() {
        let original = vec![0.0, 0.0, 0.0, 1.0];
        let processed = vec![1.0, 1.0, 1.0, 1.0];
        let out = apply_mask(&original, processed.clone(), None, 1, 1).unwrap();
        assert_eq!(out, processed);
    }

    #[test]
    fn zero_alpha_mask_reproduces_the_original() {
        let original = vec![10.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0, 1.0];
        let processed = vec![200.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0, 1.0];
        let mask = (vec![0.0, 0.0, 0.0, 0.0], 1, 1);
        let out = apply_mask(&original, processed, Some(&mask), 1, 1).unwrap();
        assert_eq!(out, original);
    }

    #[test]
    fn full_alpha_mask_reproduces_the_processed_value() {
        let original = vec![10.0 / 255.0, 20.0 / 255.0, 30.0 / 255.0, 1.0];
        let processed = vec![200.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0, 1.0];
        let mask = (vec![0.0, 0.0, 0.0, 1.0], 1, 1);
        let out = apply_mask(&original, processed.clone(), Some(&mask), 1, 1).unwrap();
        assert_eq!(out, processed);
    }

    #[test]
    fn half_alpha_mask_blends_evenly_between_original_and_processed() {
        let original = vec![0.0, 0.0, 0.0, 1.0];
        let processed = vec![1.0, 1.0, 1.0, 1.0];
        let mask = (vec![0.0, 0.0, 0.0, 0.5], 1, 1);
        let out = apply_mask(&original, processed, Some(&mask), 1, 1).unwrap();
        assert!((out[0] - 0.5).abs() < 0.001, "expected the midpoint, got {}", out[0]);
    }

    #[test]
    fn an_out_of_range_mask_weight_is_clamped_to_a_sane_blend_factor() {
        let original = vec![0.0, 0.0, 0.0, 1.0];
        let processed = vec![2.0, 0.0, 0.0, 1.0];
        // A weight above 1.0 (an out-of-gamut FloatImage wired as MASK)
        // is clamped to 1.0 - fully processed, not an extrapolation past it.
        let mask = (vec![0.0, 0.0, 0.0, 1.5], 1, 1);
        let out = apply_mask(&original, processed, Some(&mask), 1, 1).unwrap();
        assert_eq!(out[0], 2.0);
    }

    #[test]
    fn blending_toward_an_out_of_gamut_value_can_still_be_out_of_gamut() {
        // Matches how a real compositor's mix/merge preserves HDR - not
        // something to clamp mid-blend.
        let original = vec![0.0, 0.0, 0.0, 1.0];
        let processed = vec![2.0, 0.0, 0.0, 1.0];
        // 0*0.4 + 2*0.6 = 1.2 - a weight past the halfway point is needed
        // to actually land out of gamut, not just at the boundary.
        let mask = (vec![0.0, 0.0, 0.0, 0.6], 1, 1);
        let out = apply_mask(&original, processed, Some(&mask), 1, 1).unwrap();
        assert!(out[0] > 1.0, "expected an out-of-gamut blend, got {}", out[0]);
    }

    #[test]
    fn mismatched_mask_dimensions_error_instead_of_silently_ignoring_it() {
        let original = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let processed = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let mask = (vec![0.0, 0.0, 0.0, 1.0], 1, 1);
        let err = apply_mask(&original, processed, Some(&mask), 2, 1).unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn compute_within_bbox_only_calls_compute_inside_the_work_area() {
        // 4x1: work_area covers only x in [1,3).
        let original = vec![
            0.1, 0.1, 0.1, 1.0,
            0.2, 0.2, 0.2, 1.0,
            0.3, 0.3, 0.3, 1.0,
            0.4, 0.4, 0.4, 1.0,
        ];
        let work_area = Rect { x0: 1, y0: 0, x1: 3, y1: 1 };

        let out = compute_within_bbox(4, 1, work_area, &original, |_x, _y| [9.0, 9.0, 9.0, 9.0]);

        assert_eq!(&out[0..4], &[0.1, 0.1, 0.1, 1.0], "x=0 is outside the work area - must pass through original");
        assert_eq!(&out[4..8], &[9.0, 9.0, 9.0, 9.0], "x=1 is inside the work area - must be computed");
        assert_eq!(&out[8..12], &[9.0, 9.0, 9.0, 9.0], "x=2 is inside the work area - must be computed");
        assert_eq!(&out[12..16], &[0.4, 0.4, 0.4, 1.0], "x=3 is outside the work area - must pass through original");
    }

    #[test]
    fn compute_within_bbox_calls_compute_exactly_once_per_pixel_in_the_work_area() {
        let original = vec![0.0f32; 4 * 4 * 4];
        let work_area = Rect { x0: 1, y0: 1, x1: 3, y1: 3 };

        let count = std::cell::Cell::new(0u32);
        compute_within_bbox(4, 4, work_area, &original, |_x, _y| {
            count.set(count.get() + 1);
            [0.0, 0.0, 0.0, 0.0]
        });

        assert_eq!(count.get(), 4, "a 2x2 work area must call compute exactly 4 times");
    }

    #[test]
    fn compute_within_bbox_with_an_empty_work_area_never_calls_compute() {
        let original = vec![0.5f32; 4 * 4];
        let out = compute_within_bbox(4, 1, Rect::empty(), &original, |_x, _y| {
            panic!("compute must never be called for an empty work area");
        });
        assert_eq!(out, original);
    }

    #[test]
    fn compute_within_bbox_clamps_a_work_area_that_exceeds_the_frame() {
        let original = vec![0.0f32; 2 * 2 * 4];
        // Deliberately out-of-bounds on every side.
        let work_area = Rect { x0: -5, y0: -5, x1: 50, y1: 50 };

        let count = std::cell::Cell::new(0u32);
        compute_within_bbox(2, 2, work_area, &original, |_x, _y| {
            count.set(count.get() + 1);
            [1.0, 1.0, 1.0, 1.0]
        });

        assert_eq!(count.get(), 4, "an out-of-bounds work area must clamp to exactly the frame's own pixel count");
    }

    #[test]
    fn compute_within_bbox_records_the_pixel_count_for_profiling() {
        reset_pixels_computed();
        let original = vec![0.0f32; 4 * 4];
        compute_within_bbox(4, 1, Rect { x0: 1, y0: 0, x1: 3, y1: 1 }, &original, |_x, _y| [0.0, 0.0, 0.0, 0.0]);

        assert_eq!(take_pixels_computed(), Some(2));
    }

    #[test]
    fn take_pixels_computed_returns_none_when_nothing_was_recorded() {
        reset_pixels_computed();
        assert_eq!(take_pixels_computed(), None);
    }
}
