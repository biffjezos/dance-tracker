use std::sync::Arc;

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
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Rgba8,
}