use std::sync::Arc;

use crate::compositor::{Context, Operation, OperationError, Value};
use crate::operations::masks::{key_pixel, Fill};
use crate::operations::{expect_frame, Frame};

/*
inputs[0] is the video being keyed. Every pixel close enough to
key_colour (within threshold) becomes fully transparent; everything
else stays, either as a flat fill colour or the video's own colour.
Stateless - unlike Difference, there's nothing to capture.
*/
pub struct Chroma {
    pub key_colour: (u8, u8, u8),
    pub threshold: u32,
    pub fill: Fill,
}

impl Operation for Chroma {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn execute(
        &self,
        _ctx: &Context,
        inputs: &[Value],
    ) -> Result<Vec<Value>, OperationError> {
        let video = expect_frame(inputs.first())?;

        let mut pixels = Vec::with_capacity(video.pixels.len());

        for i in (0..video.pixels.len()).step_by(4) {
            let (r, g, b, a) = key_pixel(
                (video.pixels[i], video.pixels[i + 1], video.pixels[i + 2]),
                self.key_colour,
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

    fn run(op: &Chroma, video: Frame) -> Frame {
        let ctx = Context { data: Box::new(()) };
        let inputs = vec![Value::Frame(Arc::new(video))];

        let mut outputs = op.execute(&ctx, &inputs).expect("should succeed");

        match outputs.remove(0) {
            Value::Frame(frame) => (*frame).clone(),
            _ => panic!("should be a Frame"),
        }
    }

    #[test]
    fn green_screen_pixel_becomes_transparent() {
        let op = Chroma {
            key_colour: (0, 255, 0),
            threshold: 60,
            fill: Fill::Solid(255, 0, 255),
        };

        let out = run(&op, frame(vec![10, 250, 5, 255]));

        assert_eq!(out.pixels, vec![0, 0, 0, 0]);
    }

    #[test]
    fn subject_pixel_becomes_opaque_solid_fill() {
        let op = Chroma {
            key_colour: (0, 255, 0),
            threshold: 60,
            fill: Fill::Solid(255, 0, 255),
        };

        let out = run(&op, frame(vec![200, 100, 80, 255]));

        assert_eq!(out.pixels, vec![255, 0, 255, 255]);
    }

    #[test]
    fn subject_pixel_with_video_fill_keeps_its_own_colour() {
        let op = Chroma {
            key_colour: (0, 255, 0),
            threshold: 60,
            fill: Fill::Video,
        };

        let out = run(&op, frame(vec![200, 100, 80, 255]));

        assert_eq!(out.pixels, vec![200, 100, 80, 255]);
    }
}
