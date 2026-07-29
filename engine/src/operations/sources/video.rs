use std::sync::Arc;

use crate::compositor::OperationError;
use crate::graphics::Image;
use crate::operations::sources::PixelSource;

#[derive(Clone)]
pub struct Video {
    source: Arc<dyn PixelSource>,
}

impl Video {
    pub fn new(source: Arc<dyn PixelSource>) -> Self {
        Self {
            source,
        }
    }

    pub fn image_at(
        &self,
        width: u32,
        height: u32,
    ) -> Result<Arc<Image>, OperationError> {

        let frame = self.source.read(width, height)?;

        Ok(Arc::new(Image {
            pixels: frame.pixels,
            width: frame.width,
            height: frame.height,
            format: crate::graphics::ImageFormat::Rgba8,
        }))
    }
}