use crate::compositor::{Context, OperationError, Value};
use super::float_image::FloatImage;

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
}
