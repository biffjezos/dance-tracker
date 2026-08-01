use std::sync::Arc;

use super::color::Color;

#[derive(Debug, Clone)]
pub struct U8Image {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

impl U8Image {
    /// An opaque black image, used when an operation has nothing wired
    /// into an input yet and needs a placeholder at the current resolution.
    pub fn black(width: u32, height: u32) -> Arc<U8Image> {
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = 255;
        }

        Arc::new(U8Image {
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
    pub fn solid(color: Color, width: u32, height: u32) -> Arc<U8Image> {
        let rgba = color.to_rgba_u8();
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&rgba);
        }

        Arc::new(U8Image {
            pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        })
    }

    /// Resample this image into a `width`x`height` canvas, preserving
    /// aspect ratio and centering the result - the same "contain fit" a
    /// live video/camera source already gets in `VideoElementPixelSource`,
    /// so a loaded image conforms to the graph's current resolution the
    /// same way every other source does. Anything left uncovered (when
    /// the aspect ratios differ) comes out fully transparent, same as
    /// RESIZE's own out-of-bounds behaviour. Returns `self` unchanged
    /// (same Arc-worthy pixels, just re-wrapped) when the size already
    /// matches, so callers that cache by size can skip the no-op case.
    pub fn contain_fit(&self, width: u32, height: u32) -> U8Image {
        if self.width == width && self.height == height {
            return self.clone();
        }

        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        if self.width == 0 || self.height == 0 || width == 0 || height == 0 {
            return U8Image { pixels, width, height, format: self.format };
        }

        let scale = (width as f64 / self.width as f64).min(height as f64 / self.height as f64);
        let drawn_width = self.width as f64 * scale;
        let drawn_height = self.height as f64 * scale;
        let offset_x = (width as f64 - drawn_width) / 2.0;
        let offset_y = (height as f64 - drawn_height) / 2.0;

        for y in 0..height {
            for x in 0..width {
                let src_x = (x as f64 - offset_x) / scale;
                let src_y = (y as f64 - offset_y) / scale;

                if src_x < 0.0 || src_y < 0.0 || src_x >= self.width as f64 || src_y >= self.height as f64 {
                    continue;
                }

                let dest_index = ((y * width + x) * 4) as usize;
                let src_index = ((src_y as u32 * self.width + src_x as u32) * 4) as usize;
                pixels[dest_index..dest_index + 4]
                    .copy_from_slice(&self.pixels[src_index..src_index + 4]);
            }
        }

        U8Image { pixels, width, height, format: self.format }
    }

    /// The classic compositing-app "missing" placeholder - a magenta/black
    /// checker - used instead of black when an operation's input isn't
    /// wired, so a never-connected or since-removed source is visibly
    /// obvious in the actual output, not just discoverable by opening EDIT.
    pub fn missing(width: u32, height: u32) -> Arc<U8Image> {
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

        Arc::new(U8Image {
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
    fn contain_fit_returns_the_same_pixels_when_size_already_matches() {
        let image = U8Image { pixels: vec![1, 2, 3, 4], width: 1, height: 1, format: ImageFormat::Rgba8 };
        let fitted = image.contain_fit(1, 1);
        assert_eq!(fitted.pixels, image.pixels);
    }

    #[test]
    fn contain_fit_pillarboxes_a_narrower_image_into_a_wider_canvas() {
        // 1x1 opaque red into a 3x1 canvas - centered, with transparent
        // pillarbars on either side.
        let image = U8Image { pixels: vec![255, 0, 0, 255], width: 1, height: 1, format: ImageFormat::Rgba8 };
        let fitted = image.contain_fit(3, 1);

        assert_eq!(&fitted.pixels[0..4], &[0, 0, 0, 0]);
        assert_eq!(&fitted.pixels[4..8], &[255, 0, 0, 255]);
        assert_eq!(&fitted.pixels[8..12], &[0, 0, 0, 0]);
    }

    #[test]
    fn solid_fills_every_pixel_with_the_given_opaque_colour() {
        let pink = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
        let image = U8Image::solid(pink, 2, 1);
        assert_eq!(image.pixels, vec![255, 0, 255, 255, 255, 0, 255, 255]);
    }

    #[test]
    fn fully_opaque_pixels_pass_through_unchanged() {
        let pixels = vec![10, 20, 30, 255];
        let out = U8Image::composite_over_checker(&pixels, 1, 1);
        assert_eq!(out, vec![10, 20, 30, 255]);
    }

    #[test]
    fn fully_transparent_pixel_shows_the_checker_colour_underneath() {
        // (0,0) falls in the magenta tile.
        let pixels = vec![10, 20, 30, 0];
        let out = U8Image::composite_over_checker(&pixels, 1, 1);
        assert_eq!(out, vec![255, 0, 255, 255]);
    }

    #[test]
    fn half_alpha_blends_toward_the_checker() {
        let pixels = vec![0, 0, 0, 128];
        let out = U8Image::composite_over_checker(&pixels, 1, 1);
        // Blended toward magenta's red/blue channels, still fully opaque.
        assert_eq!(out[3], 255);
        assert!(out[0] > 0 && out[0] < 255);
    }
}