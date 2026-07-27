/*
The payload types every operation deals in - RGBA8, straight
(non-premultiplied) alpha, row-major, matching Canvas ImageData's byte
layout so the JS/WASM boundary is a direct copy with no repacking.

Only the categories actually implemented so far are declared here.
transforms/outputs land the same way as everything else did, one at a
time.
*/

use crate::compositor::{OperationError, Value};
use std::sync::Arc;

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
A single-channel occupancy/alpha buffer - not produced by any operation
yet (chroma/difference still produce a full Frame, matching their
existing fill:solid/video behaviour), but part of the Value enum so a
future pure-mask-producing operation has somewhere to live without
another Value variant needing to be added.
*/
#[derive(Clone, Debug)]
pub struct Mask {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/*
A static image - same byte layout as Frame, minus a timestamp (nothing
plays back). Not produced by any operation yet (sources::image is
still future work), included so Value's shape is already correct for
it.
*/
#[derive(Clone, Debug)]
pub struct Image {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/*
Every operation that reads a Frame input needs this exact match -
factored out once here instead of copy-pasted in compose/apply_mask/
chroma/difference/rings/ghost/text. Returns a plain &Frame for the
common "just read it this call" case; expect_frame_arc below is for
the rarer "I need to keep this around" case (Ghost's history,
CapturedFrame's captured background), where cloning the Arc instead of
the pixels is the entire point of Value carrying one.
*/
pub fn expect_frame(value: Option<&Value>) -> Result<&Frame, OperationError> {
    match value {
        Some(Value::Frame(frame)) => Ok(frame.as_ref()),
        Some(_) => Err(OperationError::WrongValueType),
        None => Err(OperationError::MissingInput),
    }
}

pub fn expect_frame_arc(value: Option<&Value>) -> Result<Arc<Frame>, OperationError> {
    match value {
        Some(Value::Frame(frame)) => Ok(frame.clone()),
        Some(_) => Err(OperationError::WrongValueType),
        None => Err(OperationError::MissingInput),
    }
}

pub mod animations;
pub mod sources;
pub mod composite;
pub mod generators;
pub mod masks;
pub mod executor;
