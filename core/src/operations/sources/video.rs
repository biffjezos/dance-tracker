use crate::compositor::{Context, Operation, OperationError, Value};
use crate::operations::sources::PixelSource;

pub struct VideoSource {
    pub pixels: Box<dyn PixelSource>,
}

impl Operation for VideoSource {
    fn execute(
        &self,
        _ctx: &Context,
        _inputs: &[Box<dyn Value>],
    ) -> Result<Vec<Box<dyn Value>>, OperationError> {
        let frame = self.pixels.read()?;

        Ok(vec![Box::new(frame)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::Frame;
    use std::any::Any;

    struct FixedPixelSource(Frame);

    impl PixelSource for FixedPixelSource {
        fn read(&self) -> Result<Frame, OperationError> {
            Ok(self.0.clone())
        }
    }

    struct FailingPixelSource;

    impl PixelSource for FailingPixelSource {
        fn read(&self) -> Result<Frame, OperationError> {
            Err(OperationError::SourceNotFound("gone".to_string()))
        }
    }

    #[test]
    fn returns_whatever_its_pixel_source_provides() {
        let node = VideoSource {
            pixels: Box::new(FixedPixelSource(Frame {
                pixels: vec![1, 2, 3, 4],
                width: 1,
                height: 1,
                timestamp: 0.0,
            })),
        };

        let ctx = Context { data: Box::new(()) };

        let mut outputs = node.execute(&ctx, &[]).expect("should succeed");
        let any_box: Box<dyn Any> = outputs.remove(0);
        let frame = any_box.downcast::<Frame>().expect("should be a Frame");

        assert_eq!(frame.pixels, vec![1, 2, 3, 4]);
    }

    #[test]
    fn propagates_the_pixel_sources_error_instead_of_panicking() {
        let node = VideoSource {
            pixels: Box::new(FailingPixelSource),
        };

        let ctx = Context { data: Box::new(()) };

        let result = node.execute(&ctx, &[]);

        assert!(matches!(result, Err(OperationError::SourceNotFound(_))));
    }
}
