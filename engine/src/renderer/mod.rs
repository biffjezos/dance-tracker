// src/renderer/mod.rs
// Renderer boundary module
//
// This module provides the rendering boundary abstraction for converting
// various graph value types into renderable Frames for display.
//
// Design principles:
// - Graph nodes output their natural types (Image, Frame, Mask, etc.)
// - No automatic conversions occur during graph evaluation
// - All rendering-specific adaptation happens here, at the renderer boundary
// - New renderable types can be added by implementing ToRenderFrame

use std::sync::Arc;
use crate::compositor::Value;
use crate::graphics::{Frame, Image, mask::Mask, video::Video};
use crate::compositor::OperationError;

/// Trait for types that can be converted to a Frame for rendering.
/// 
/// Implement this trait for any Value variant that should be renderable.
/// This keeps the renderer extensible - new types just need to implement
/// this trait to be supported by the rendering pipeline.
pub trait ToRenderFrame {
    /// Convert this value to a Frame suitable for rendering.
    /// 
    /// For types that already are Frames, this is a no-op (clone).
    /// For types like Image that need conversion, this handles the adaptation.
    fn to_render_frame(&self) -> Result<Arc<Frame>, OperationError>;
}

/// Convert a Value to a Frame for rendering.
/// 
/// This is the renderer boundary dispatch - it delegates to the appropriate
/// ToRenderFrame implementation based on the Value variant.
/// 
/// This function is ONLY called by the renderer (preview_tick, render_tick).
/// Graph operations should work with their natural Value types directly.
pub fn to_render_frame(value: &Value) -> Result<Arc<Frame>, OperationError> {
    match value {
        Value::Frame(frame) => frame.to_render_frame(),
        Value::Image(image) => image.to_render_frame(),
        Value::Mask(mask) => mask.to_render_frame(),
        Value::Video(video) => video.to_render_frame(),
        _ => Err(OperationError::WrongValueType),
    }
}

// Implement ToRenderFrame for Frame (identity conversion)
impl ToRenderFrame for Arc<Frame> {
    fn to_render_frame(&self) -> Result<Arc<Frame>, OperationError> {
        Ok(self.clone())
    }
}

// Implement ToRenderFrame for Image (convert to Frame with timestamp 0.0)
impl ToRenderFrame for Arc<Image> {
    fn to_render_frame(&self) -> Result<Arc<Frame>, OperationError> {
        Ok(Arc::new(Frame {
            pixels: self.pixels.clone(),
            width: self.width,
            height: self.height,
            timestamp: 0.0, // Still images have no temporal information
        }))
    }
}

// Implement ToRenderFrame for Mask (convert to Frame)
// Masks are typically single-channel, but we treat them as RGBA for rendering
impl ToRenderFrame for Arc<Mask> {
    fn to_render_frame(&self) -> Result<Arc<Frame>, OperationError> {
        // For now, convert mask to a grayscale frame
        // In the future, this could be more sophisticated (e.g., use mask as alpha channel)
        let mut pixels = Vec::with_capacity((self.width * self.height * 4) as usize);
        
        for &byte in &self.pixels {
            // Convert single-channel mask to RGBA (replicate to all channels)
            pixels.push(byte);     // R
            pixels.push(byte);     // G
            pixels.push(byte);     // B
            pixels.push(byte);     // A
        }
        
        Ok(Arc::new(Frame {
            pixels,
            width: self.width,
            height: self.height,
            timestamp: 0.0,
        }))
    }
}

// Implement ToRenderFrame for Video (extract current frame)
// For now, we'll extract the first frame. In the future, this should use the current time.
impl ToRenderFrame for Arc<Video> {
    fn to_render_frame(&self) -> Result<Arc<Frame>, OperationError> {
        // Get the first frame from the video
        // In a real implementation, this would use the current playback time
        // to select the appropriate frame
        if self.frames.is_empty() {
            return Err(OperationError::SourceNotFound("Video contains no frames".to_string()));
        }
        
        // For now, just convert the first image to a frame
        let first_image = &self.frames[0];
        Ok(Arc::new(Frame {
            pixels: first_image.pixels.clone(),
            width: first_image.width,
            height: first_image.height,
            timestamp: 0.0,
        }))
    }
}
