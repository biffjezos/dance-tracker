/*
The one payload type every pixel-producing operation deals in - RGBA8,
straight (non-premultiplied) alpha, row-major, matching Canvas
ImageData's byte layout so the eventual JS/WASM boundary is a direct
copy with no repacking.

Only the categories actually implemented so far are declared here.
generators/masks/transforms/controls/outputs land the same way as
sources and composite did, one at a time.
*/

#[derive(Clone, Debug)]
pub struct Frame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: f64,
}

impl Frame {
    pub fn same_dimensions(&self, other: &Frame) -> bool {
        self.width == other.width && self.height == other.height
    }
}

pub mod sources;
pub mod composite;
pub mod executor;
