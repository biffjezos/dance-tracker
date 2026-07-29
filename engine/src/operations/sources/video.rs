use std::sync::Arc;

use crate::compositor::OperationError;
use crate::graphics::Video;
use crate::operations::sources::PixelSource;

#[derive(Clone)]
pub struct VideoSource {
    video: Arc<Video>,
}

impl VideoSource {
    pub fn new(video: Arc<Video>) -> Self {
        Self {
            video,
        }
    }

    pub fn video(&self) -> Arc<Video> {
        self.video.clone()
    }
}