use std::sync::Arc;

use crate::compositor::OperationError;
use crate::graphics::U8Image;

#[derive(Clone, Debug)]
pub struct Video {
    pub frames: Vec<Arc<U8Image>>,
    pub fps: f32,
}

impl Video {
    pub fn new(frames: Vec<Arc<U8Image>>, fps: f32) -> Self {
        Self {
            frames,
            fps,
        }
    }

    pub fn frame_at(&self, time: f64) -> Result<Arc<U8Image>, OperationError> {
        if self.frames.is_empty() {
            return Err(OperationError::SourceNotFound(
                "Video contains no frames".to_string()
            ));
        }

        if time <= 0.0 {
            return Ok(self.frames[0].clone());
        }

        let index = (time * self.fps as f64).floor() as usize;

        if index >= self.frames.len() {
            return Ok(self.frames[self.frames.len() - 1].clone());
        }

        Ok(self.frames[index].clone())
    }
}
