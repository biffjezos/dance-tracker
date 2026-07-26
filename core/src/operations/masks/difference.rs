use std::sync::Arc;

use crate::compositor::{Context, Operation, OperationError, Value};
use crate::operations::masks::{key_pixel, Fill};
use crate::operations::{expect_frame, Frame};

/*
inputs[0] is the video being keyed, inputs[1] is the reference/
"empty room" frame - per-pixel, not a single colour like Chroma. This
operation itself is stateless; "CAPTURE BACKGROUND" in the UI is pure
node-graph rewiring (see sources::captured::CapturedFrame), not
something this operation does internally - wire a CapturedFrame node
into inputs[1] and repoint/recapture it whenever the user clicks
CAPTURE BACKGROUND. A live second feed works here too, for true
real-time background subtraction, which the old fixed-snapshot-only
version couldn't do.
*/
pub struct Difference {
    pub threshold: u32,
    pub fill: Fill,
}

impl Operation for Difference {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn execute(
        &self,
        _ctx: &Context,
        inputs: &[Value],
    ) -> Result<Vec<Value>, OperationError> {
        let video = expect_frame(inputs.first())?;
        let reference = expect_frame(inputs.get(1))?;

        if !video.same_dimensions(reference) {
            return Err(OperationError::DimensionMismatch);
        }

        let mut pixels = Vec::with_capacity(video.pixels.len());

        for i in (0..video.pixels.len()).step_by(4) {
            let (r, g, b, a) = key_pixel(
                (video.pixels[i], video.pixels[i + 1], video.pixels[i + 2]),
                (reference.pixels[i], reference.pixels[i + 1], reference.pixels[i + 2]),
                self.threshold,
                self.fill,
            );

            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }

        let frame = Frame {
            pixels,
            width: video.width,
            height: video.height,
            timestamp: video.timestamp,
        };

        Ok(vec![Value::Frame(Arc::new(frame))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(pixels: Vec<u8>) -> Frame {
        Frame { pixels, width: 1, height: 1, timestamp: 0.0 }
    }

    fn run(op: &Difference, video: Frame, reference: Frame) -> Frame {
        let ctx = Context::default();
        let inputs = vec![Value::Frame(Arc::new(video)), Value::Frame(Arc::new(reference))];

        let mut outputs = op.execute(&ctx, &inputs).expect("should succeed");

        match outputs.remove(0) {
            Value::Frame(frame) => (*frame).clone(),
            _ => panic!("should be a Frame"),
        }
    }

    #[test]
    fn pixel_matching_captured_background_becomes_transparent() {
        let op = Difference { threshold: 30, fill: Fill::Solid(255, 0, 255) };

        let out = run(&op, frame(vec![40, 40, 40, 255]), frame(vec![42, 41, 39, 255]));

        assert_eq!(out.pixels, vec![0, 0, 0, 0]);
    }

    #[test]
    fn pixel_differing_from_captured_background_becomes_opaque() {
        let op = Difference { threshold: 30, fill: Fill::Solid(255, 0, 255) };

        let out = run(&op, frame(vec![200, 20, 20, 255]), frame(vec![10, 10, 10, 255]));

        assert_eq!(out.pixels, vec![255, 0, 255, 255]);
    }

    #[test]
    fn mismatched_dimensions_error_instead_of_panicking() {
        let op = Difference { threshold: 30, fill: Fill::Video };
        let ctx = Context::default();
        let video = frame(vec![1, 2, 3, 255]);
        let reference = Frame {
            pixels: vec![0, 0, 0, 255, 0, 0, 0, 255],
            width: 2,
            height: 1,
            timestamp: 0.0,
        };
        let inputs = vec![Value::Frame(Arc::new(video)), Value::Frame(Arc::new(reference))];

        let result = op.execute(&ctx, &inputs);

        assert!(matches!(result, Err(OperationError::DimensionMismatch)));
    }
}
