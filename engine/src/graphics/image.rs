use std::sync::Arc;

use super::color::Color;

#[derive(Debug, Clone)]
pub struct Image {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

impl Image {
    /// An opaque black image, used when an operation has nothing wired
    /// into an input yet and needs a placeholder at the current resolution.
    pub fn black(width: u32, height: u32) -> Arc<Image> {
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = 255;
        }

        Arc::new(Image {
            pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        })
    }

    /// An opaque image filled with a single solid colour - an alternative
    /// "unconnected input" placeholder for operations where the busy
    /// missing()/transparency checker is more confusing than helpful (e.g.
    /// a mask-producing node, where there's nothing to key "removal"
    /// against, and a checker pattern is easy to mistake for real content
    /// when eyedropping a colour off the canvas).
    pub fn solid(color: Color, width: u32, height: u32) -> Arc<Image> {
        let rgba = color.to_rgba_u8();
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&rgba);
        }

        Arc::new(Image {
            pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        })
    }

    /// The classic compositing-app "missing" placeholder - a magenta/black
    /// checker - used instead of black when an operation's input isn't
    /// wired, so a never-connected or since-removed source is visibly
    /// obvious in the actual output, not just discoverable by opening EDIT.
    pub fn missing(width: u32, height: u32) -> Arc<Image> {
        const TILE: u32 = 16;
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        for y in 0..height {
            for x in 0..width {
                let checker = ((x / TILE) + (y / TILE)) % 2 == 0;
                let index = ((y * width + x) * 4) as usize;
                let color: [u8; 4] = if checker {
                    [255, 0, 255, 255]
                } else {
                    [0, 0, 0, 255]
                };
                pixels[index..index + 4].copy_from_slice(&color);
            }
        }

        Arc::new(Image {
            pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        })
    }

    /// Alpha-composite an RGBA buffer over the same magenta/black checker
    /// used by `missing()` (at half the tile size, so it reads as a
    /// distinct "transparency grid" rather than "missing input"), for
    /// display only - lets the user see exactly where a node's output is
    /// actually transparent, rather than however the canvas element's own
    /// background happens to render it. The result is always fully opaque
    /// (alpha 255), since it's the final thing drawn to screen.
    pub fn composite_over_checker(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
        const TILE: u32 = 8;
        let mut output = vec![0u8; pixels.len()];

        for y in 0..height {
            for x in 0..width {
                let index = ((y * width + x) * 4) as usize;
                let checker = ((x / TILE) + (y / TILE)) % 2 == 0;
                let background: [f32; 3] = if checker {
                    [255.0, 0.0, 255.0]
                } else {
                    [0.0, 0.0, 0.0]
                };

                let alpha = pixels[index + 3] as f32 / 255.0;

                for channel in 0..3 {
                    let foreground = pixels[index + channel] as f32;
                    output[index + channel] =
                        (foreground * alpha + background[channel] * (1.0 - alpha)).round() as u8;
                }
                output[index + 3] = 255;
            }
        }

        output
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Rgba8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_fills_every_pixel_with_the_given_opaque_colour() {
        let pink = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
        let image = Image::solid(pink, 2, 1);
        assert_eq!(image.pixels, vec![255, 0, 255, 255, 255, 0, 255, 255]);
    }

    #[test]
    fn fully_opaque_pixels_pass_through_unchanged() {
        let pixels = vec![10, 20, 30, 255];
        let out = Image::composite_over_checker(&pixels, 1, 1);
        assert_eq!(out, vec![10, 20, 30, 255]);
    }

    #[test]
    fn fully_transparent_pixel_shows_the_checker_colour_underneath() {
        // (0,0) falls in the magenta tile.
        let pixels = vec![10, 20, 30, 0];
        let out = Image::composite_over_checker(&pixels, 1, 1);
        assert_eq!(out, vec![255, 0, 255, 255]);
    }

    #[test]
    fn half_alpha_blends_toward_the_checker() {
        let pixels = vec![0, 0, 0, 128];
        let out = Image::composite_over_checker(&pixels, 1, 1);
        // Blended toward magenta's red/blue channels, still fully opaque.
        assert_eq!(out[3], 255);
        assert!(out[0] > 0 && out[0] < 255);
    }
}