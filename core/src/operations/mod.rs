/*
The one payload type every pixel-producing operation deals in - RGBA8,
straight (non-premultiplied) alpha, row-major, matching Canvas
ImageData's byte layout so the eventual JS/WASM boundary is a direct
copy with no repacking.

Only the categories actually implemented so far are declared here.
generators/masks/transforms/controls/outputs land the same way as
sources and composite did, one at a time.
*/

use crate::compositor::{OperationError, Value};
use std::any::Any;

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

    pub fn blank(width: u32, height: u32, timestamp: f64) -> Frame {
        Frame {
            pixels: vec![0; (width as usize) * (height as usize) * 4],
            width,
            height,
            timestamp,
        }
    }
}

/*
Every operation that reads a Frame input needs this exact downcast -
factored out once here instead of copy-pasted in compose/apply_mask/
chroma/difference/rings/ghost/text.
*/
pub fn downcast_frame(value: Option<&Box<dyn Value>>) -> Result<&Frame, OperationError> {
    let value = value.ok_or(OperationError::MissingInput)?;

    let any_ref: &dyn Any = value.as_ref();

    any_ref
        .downcast_ref::<Frame>()
        .ok_or(OperationError::WrongValueType)
}

pub mod sources;
pub mod composite;
pub mod generators;
pub mod masks;
pub mod controls;
pub mod executor;
