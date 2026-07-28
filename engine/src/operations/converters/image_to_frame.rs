// src/operations/converters/image_to_frame.rs
use std::sync::Arc;

use crate::graphics::{Frame, Image};

/// Convert an Image to a Frame
/// Images don't have timestamps, so we use 0.0
pub fn image_to_frame(image: &Image) -> Frame {
    Frame {
        pixels: image.pixels.clone(),
        width: image.width,
        height: image.height,
        timestamp: 0.0,
    }
}

/// Create an Arc<Frame> from an Arc<Image>
pub fn arc_image_to_arc_frame(image: Arc<Image>) -> Arc<Frame> {
    Arc::new(image_to_frame(&image))
}
