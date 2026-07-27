use std::sync::Arc;

use crate::compositor::{Context, Input, Operation, OperationError, Value};
use crate::operations::sources::PixelSource;

pub struct VideoSource {
    pub pixels: Box<dyn PixelSource>,
}

impl Operation for VideoSource {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn execute(
        &self,
        ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        let frame = self.pixels.read(ctx.meta.width, ctx.meta.height)?;

        Ok(vec![Value::Frame(Arc::new(frame))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::Frame;

    struct FixedPixelSource(Frame);

    impl PixelSource for FixedPixelSource {
        fn read(&self, _width: u32, _height: u32) -> Result<Frame, OperationError> {
            Ok(self.0.clone())
        }
    }

    struct FailingPixelSource;

    impl PixelSource for FailingPixelSource {
        fn read(&self, _width: u32, _height: u32) -> Result<Frame, OperationError> {
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

        let ctx = Context::default();

        let mut outputs = node.execute(&ctx, &[]).expect("should succeed");

        let frame = match outputs.remove(0) {
            Value::Frame(frame) => frame,
            _ => panic!("should be a Frame"),
        };

        assert_eq!(frame.pixels, vec![1, 2, 3, 4]);
    }

    #[test]
    fn propagates_the_pixel_sources_error_instead_of_panicking() {
        let node = VideoSource {
            pixels: Box::new(FailingPixelSource),
        };

        let ctx = Context::default();

        let result = node.execute(&ctx, &[]);

        assert!(matches!(result, Err(OperationError::SourceNotFound(_))));
    }
}
