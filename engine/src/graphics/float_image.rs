use crate::compositor::{Context, OperationError, Value};
use super::u8_image::{U8Image, ImageFormat};

/// An RGBA image whose channels are NOT bounded to 0.0..1.0 - the
/// unclamped counterpart to `U8Image` (u8, always 0..255 by construction,
/// so there is no way to represent an out-of-range value in it at all).
///
/// Operations like ADD/SUBTRACT produce this instead of U8Image so an
/// over/under-range result - what a real compositor calls "out of gamut"
/// (an overexposed highlight, a difference that goes negative) - survives
/// downstream math instead of being silently clipped the instant it's
/// computed. This mirrors how Nuke/Fusion/Natron etc. work internally
/// (unbounded float, clip only where an artist explicitly asks for it),
/// rather than clamping-by-default the way an 8-bit-only pipeline would.
///
/// CLAMP (`operations::transform::clamp`) is the explicit, deliberate step
/// back down to a normal bounded U8Image - nothing does that conversion
/// silently.
#[derive(Debug, Clone)]
pub struct FloatImage {
    pub pixels: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

impl FloatImage {
    /// Build from a normal (already-bounded) U8Image - every channel already
    /// sits in 0.0..1.0, just represented at higher precision so it can be
    /// combined with genuinely out-of-range data without a premature clamp.
    pub fn from_image(image: &U8Image) -> Self {
        FloatImage {
            pixels: image.pixels.iter().map(|&c| c as f32 / 255.0).collect(),
            width: image.width,
            height: image.height,
        }
    }

    /// Read any pixel-bearing Value as float RGBA - the one place every
    /// operation reads its wired pixel input from, so accepting a
    /// FloatImage (out-of-gamut or not) alongside a normal bounded
    /// U8Image never needs per-operation special-casing. A bounded
    /// U8Image's channels are simply already 0.0..1.0; an already-
    /// unbounded FloatImage passes through untouched - this never clamps.
    pub fn from_value(value: &Value, ctx: &Context) -> Result<FloatImage, OperationError> {
        match value {
            Value::FloatImage(float_image) => Ok((**float_image).clone()),
            Value::Image(image) => Ok(FloatImage::from_image(image)),
            Value::Frame(frame) => Ok(FloatImage::from_image(&U8Image {
                pixels: frame.pixels.clone(),
                width: frame.width,
                height: frame.height,
                format: ImageFormat::Rgba8,
            })),
            Value::Video(video) => {
                let image = video.frame_at(ctx.meta.time)?;
                Ok(FloatImage::from_image(&image))
            }
            other => Err(OperationError::InvalidInputType(
                format!("Expected pixel data, got {:?}", other)
            )),
        }
    }

    /// Clamp every channel to `min..max` and quantize back down to a normal
    /// bounded U8Image. The one place an out-of-gamut value actually gets
    /// thrown away - and only when something (CLAMP, or a render boundary
    /// as a last resort for display) explicitly asks for it.
    pub fn to_image_clamped(&self, min: f32, max: f32) -> U8Image {
        U8Image {
            pixels: self
                .pixels
                .iter()
                .map(|&c| (c.clamp(min, max) * 255.0).round() as u8)
                .collect(),
            width: self.width,
            height: self.height,
            format: ImageFormat::Rgba8,
        }
    }

    /// Whether any pixel's R, G, or B channel - alpha is coverage, not a
    /// radiometric value, so it isn't a "gamut" concept the same way -
    /// falls outside 0.0..1.0. What the render boundary uses to decide
    /// whether to surface the "node out of gamut" warning.
    pub fn is_out_of_gamut(&self) -> bool {
        self.pixels
            .chunks_exact(4)
            .any(|px| px[0..3].iter().any(|&c| !(0.0..=1.0).contains(&c)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn context() -> Context {
        Context::default()
    }

    #[test]
    fn from_value_normalizes_a_bounded_image() {
        let image = Arc::new(U8Image { pixels: vec![0, 128, 255, 255], width: 1, height: 1, format: ImageFormat::Rgba8 });
        let float_image = FloatImage::from_value(&Value::Image(image), &context()).unwrap();
        assert_eq!(float_image.pixels[2], 1.0);
    }

    #[test]
    fn from_value_passes_an_already_unbounded_float_image_through_untouched() {
        let original = Arc::new(FloatImage { pixels: vec![1.5, -0.2, 0.5, 1.0], width: 1, height: 1 });
        let float_image = FloatImage::from_value(&Value::FloatImage(original.clone()), &context()).unwrap();
        assert_eq!(float_image.pixels, original.pixels);
    }

    #[test]
    fn from_value_rejects_a_non_pixel_value() {
        let err = FloatImage::from_value(&Value::Number(1.0), &context()).unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn from_image_normalizes_u8_channels_to_0_1() {
        let image = U8Image { pixels: vec![0, 128, 255, 255], width: 1, height: 1, format: ImageFormat::Rgba8 };
        let float_image = FloatImage::from_image(&image);
        assert_eq!(float_image.pixels[0], 0.0);
        assert!((float_image.pixels[1] - 0.502).abs() < 0.01);
        assert_eq!(float_image.pixels[2], 1.0);
        assert_eq!(float_image.pixels[3], 1.0);
    }

    #[test]
    fn to_image_clamped_clips_out_of_range_channels() {
        let float_image = FloatImage { pixels: vec![-0.5, 1.5, 0.5, 1.0], width: 1, height: 1 };
        let image = float_image.to_image_clamped(0.0, 1.0);
        assert_eq!(image.pixels, vec![0, 255, 128, 255]);
    }

    #[test]
    fn to_image_clamped_respects_a_custom_range() {
        // A creative clip: crush anything below 0.2, allow up to 1.5
        // (still representable, just quantized - the final u8 cast
        // saturates on its own for anything genuinely unrepresentable).
        let float_image = FloatImage { pixels: vec![0.1, 1.5, 0.5, 1.0], width: 1, height: 1 };
        let image = float_image.to_image_clamped(0.2, 1.5);
        assert_eq!(image.pixels[0], 51); // 0.2 * 255, rounded
        assert_eq!(image.pixels[1], 255); // 1.5 clamped to 1.5, *255 saturates to 255 in the u8 cast
    }

    #[test]
    fn in_gamut_pixels_are_not_flagged() {
        let float_image = FloatImage { pixels: vec![0.0, 0.5, 1.0, 1.0], width: 1, height: 1 };
        assert!(!float_image.is_out_of_gamut());
    }

    #[test]
    fn an_over_range_channel_is_flagged_out_of_gamut() {
        let float_image = FloatImage { pixels: vec![0.0, 0.5, 1.2, 1.0], width: 1, height: 1 };
        assert!(float_image.is_out_of_gamut());
    }

    #[test]
    fn an_under_range_channel_is_flagged_out_of_gamut() {
        let float_image = FloatImage { pixels: vec![-0.1, 0.5, 1.0, 1.0], width: 1, height: 1 };
        assert!(float_image.is_out_of_gamut());
    }

    #[test]
    fn an_out_of_range_alpha_alone_is_not_flagged() {
        // Alpha isn't a gamut concept - only R/G/B are checked.
        let float_image = FloatImage { pixels: vec![0.0, 0.5, 1.0, 2.0], width: 1, height: 1 };
        assert!(!float_image.is_out_of_gamut());
    }
}
