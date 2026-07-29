pub mod camera;
pub mod image;
pub mod video;

use crate::{
    compositor::error::OperationError,
    operations::Frame,
};

pub use image::ImageSource;
pub use video::VideoSource;

pub trait PixelSource {
    fn read(
        &self,
        width: u32,
        height: u32,
    ) -> Result<Frame, OperationError>;
}
