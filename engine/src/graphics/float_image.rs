use super::image::{Image, ImageFormat};

/// An RGBA image whose channels are NOT bounded to 0.0..1.0 - the
/// unclamped counterpart to `Image` (u8, always 0..255 by construction,
/// so there is no way to represent an out-of-range value in it at all).
///
/// Operations like ADD/SUBTRACT produce this instead of Image so an
/// over/under-range result - what a real compositor calls "out of gamut"
/// (an overexposed highlight, a difference that goes negative) - survives
/// downstream math instead of being silently clipped the instant it's
/// computed. This mirrors how Nuke/Fusion/Natron etc. work internally
/// (unbounded float, clip only where an artist explicitly asks for it),
/// rather than clamping-by-default the way an 8-bit-only pipeline would.
///
/// CLAMP (`operations::transform::clamp`) is the explicit, deliberate step
/// back down to a normal bounded Image - nothing does that conversion
/// silently.
#[derive(Debug, Clone)]
pub struct FloatImage {
    pub pixels: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

impl FloatImage {
    /// Build from a normal (already-bounded) Image - every channel already
    /// sits in 0.0..1.0, just represented at higher precision so it can be
    /// combined with genuinely out-of-range data without a premature clamp.
    pub fn from_image(image: &Image) -> Self {
        FloatImage {
            pixels: image.pixels.iter().map(|&c| c as f32 / 255.0).collect(),
            width: image.width,
            height: image.height,
        }
    }

    /// Clamp every channel to `min..max` and quantize back down to a normal
    /// u8 Image. The one place an out-of-gamut value actually gets thrown
    /// away - and only when something (CLAMP, or a render boundary as a
    /// last resort for display) explicitly asks for it.
    pub fn to_image_clamped(&self, min: f32, max: f32) -> Image {
        Image {
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

    #[test]
    fn from_image_normalizes_u8_channels_to_0_1() {
        let image = Image { pixels: vec![0, 128, 255, 255], width: 1, height: 1, format: ImageFormat::Rgba8 };
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
