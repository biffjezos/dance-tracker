#[derive(Debug, Clone)]
pub struct Image {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Rgba8,
}