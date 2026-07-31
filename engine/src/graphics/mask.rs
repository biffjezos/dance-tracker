use crate::compositor::{Context, OperationError, Value};

#[derive(Debug)]
pub struct Mask {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Resolve an operation's wired MASK input to RGBA pixels + dimensions -
/// the same Frame/Image/Video normalization every operation already
/// repeats for its own SOURCE, reused here so each operation doesn't have
/// to duplicate it a second time just to read a mask.
pub fn resolve_mask_pixels(value: &Value, ctx: &Context) -> Result<(Vec<u8>, u32, u32), OperationError> {
    match value {
        Value::Frame(frame) => Ok((frame.pixels.clone(), frame.width, frame.height)),
        Value::Image(image) => Ok((image.pixels.clone(), image.width, image.height)),
        Value::Video(video) => {
            let image = video.frame_at(ctx.meta.time)?;
            Ok((image.pixels.clone(), image.width, image.height))
        }
        other => Err(OperationError::InvalidInputType(format!(
            "MASK must be a pixel-bearing value, got {:?}",
            other
        ))),
    }
}

/// Blend `processed` toward `original` per pixel, weighted by the wired
/// mask's own alpha channel (0 = fully original/identity, 255 = fully
/// processed) - the one shared mechanism behind every operation's optional
/// MASK input, so the blend itself is implemented exactly once rather than
/// once per operation. `mask` is `(pixels, width, height)`; passing `None`
/// (nothing wired) returns `processed` untouched.
pub fn apply_mask(
    original: &[u8],
    processed: Vec<u8>,
    mask: Option<&(Vec<u8>, u32, u32)>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, OperationError> {
    let Some((mask_pixels, mask_width, mask_height)) = mask else {
        return Ok(processed);
    };

    if *mask_width != width || *mask_height != height {
        return Err(OperationError::InvalidInputType(format!(
            "MASK is {}x{}, but the node it's masking is {}x{}",
            mask_width, mask_height, width, height
        )));
    }

    let mut out = vec![0u8; processed.len()];
    for i in (0..processed.len()).step_by(4) {
        let weight = mask_pixels[i + 3] as f32 / 255.0;
        for c in 0..4 {
            let o = original[i + c] as f32;
            let p = processed[i + c] as f32;
            out[i + c] = (o * (1.0 - weight) + p * weight).round() as u8;
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_mask_returns_processed_unchanged() {
        let original = vec![0, 0, 0, 255];
        let processed = vec![255, 255, 255, 255];
        let out = apply_mask(&original, processed.clone(), None, 1, 1).unwrap();
        assert_eq!(out, processed);
    }

    #[test]
    fn zero_alpha_mask_reproduces_the_original() {
        let original = vec![10, 20, 30, 255];
        let processed = vec![200, 200, 200, 255];
        let mask = (vec![0, 0, 0, 0], 1, 1);
        let out = apply_mask(&original, processed, Some(&mask), 1, 1).unwrap();
        assert_eq!(out, original);
    }

    #[test]
    fn full_alpha_mask_reproduces_the_processed_value() {
        let original = vec![10, 20, 30, 255];
        let processed = vec![200, 200, 200, 255];
        let mask = (vec![0, 0, 0, 255], 1, 1);
        let out = apply_mask(&original, processed.clone(), Some(&mask), 1, 1).unwrap();
        assert_eq!(out, processed);
    }

    #[test]
    fn half_alpha_mask_blends_evenly_between_original_and_processed() {
        let original = vec![0, 0, 0, 255];
        let processed = vec![200, 200, 200, 255];
        let mask = (vec![0, 0, 0, 128], 1, 1);
        let out = apply_mask(&original, processed, Some(&mask), 1, 1).unwrap();
        // 128/255 ~= 0.502 -> 0*0.498 + 200*0.502 ~= 100
        assert!(out[0] >= 98 && out[0] <= 102, "expected roughly the midpoint, got {}", out[0]);
    }

    #[test]
    fn mismatched_mask_dimensions_error_instead_of_silently_ignoring_it() {
        let original = vec![0, 0, 0, 255, 0, 0, 0, 255];
        let processed = vec![200, 200, 200, 255, 200, 200, 200, 255];
        let mask = (vec![0, 0, 0, 255], 1, 1);
        let err = apply_mask(&original, processed, Some(&mask), 2, 1).unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }
}
