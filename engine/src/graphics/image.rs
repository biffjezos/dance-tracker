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
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Rgba8,
}