use crate::graphics::frame::Frame;

pub struct PixelSource {
    frame: Frame,
}

impl PixelSource {
    pub fn new(frame: Frame) -> Self {
        Self {
            frame,
        }
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn frame_mut(&mut self) -> &mut Frame {
        &mut self.frame
    }

    pub fn into_frame(self) -> Frame {
        self.frame
    }
}