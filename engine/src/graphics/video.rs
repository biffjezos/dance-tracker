// src/graphics/video.rs

use std::sync::Arc;

use crate::compositor::OperationError;
use crate::graphics::Image;

#[derive(Clone)]
pub struct Video {
    pub frames: Vec<Arc<Image>>,
    pub fps: f32,
}

impl Video {
    pub fn new(frames: Vec<Arc<Image>>, fps: f32) -> Self {
        Self {
            frames,
            fps,
        }
    }

    pub fn frame_at(&self, time: f64) -> Result<Arc<Image>, OperationError> {
        if self.frames.is_empty() {
            return Err(OperationError::SourceNotFound(
                "Video contains no frames".to_string()
            ));
        }

        let index = (time * self.fps as f64).floor() as usize;

        let index = index.min(self.frames.len() - 1);

        Ok(self.frames[index].clone())
    }
}