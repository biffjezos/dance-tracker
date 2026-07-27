use std::sync::Arc;

use crate::compositor::{
    find_input, Context, Input, Operation, OperationError, ParameterDescriptor, ParameterKind,
    Value,
};
use crate::operations::masks::{key_pixel, Fill};
use crate::operations::{expect_frame, Frame};

/*
Input::Source is the video being keyed. Every pixel close enough to
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

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![ParameterDescriptor { name: "threshold", kind: ParameterKind::Number }]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "threshold" => Some(Value::Number(self.threshold as f64)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("threshold", Value::Number(v)) => { self.threshold = v.max(0.0) as u32; Ok(()) }
            ("threshold", _) => Err(OperationError::WrongValueType),
            _ => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    fn execute(
        &self,
        _ctx: &Context,
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        let video = expect_frame(find_input(inputs, Input::Source))?;

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
        let ctx = Context::default();
        let inputs = vec![(Input::Source, Value::Frame(Arc::new(video)))];

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

    #[test]
    fn threshold_parameter_round_trips_and_takes_effect() {
        let mut op = Chroma {
            key_colour: (0, 255, 0),
            threshold: 60,
            fill: Fill::Solid(255, 0, 255),
        };

        assert!(matches!(op.get_parameter("threshold"), Some(Value::Number(v)) if v == 60.0));

        op.set_parameter("threshold", Value::Number(5.0)).expect("should accept a Number");

        assert_eq!(op.threshold, 5);

        // A pixel that was within the old threshold (60) of the key
        // colour no longer is, now that threshold has been lowered to 5.
        let out = run(&op, frame(vec![10, 250, 5, 255]));
        assert_ne!(out.pixels, vec![0, 0, 0, 0], "lowered threshold should stop keying this pixel out");
    }

    #[test]
    fn set_parameter_rejects_wrong_type_and_unknown_name() {
        let mut op = Chroma {
            key_colour: (0, 255, 0),
            threshold: 60,
            fill: Fill::Solid(255, 0, 255),
        };

        assert!(matches!(
            op.set_parameter("threshold", Value::Text("nope".to_string())),
            Err(OperationError::WrongValueType)
        ));

        assert!(matches!(
            op.set_parameter("not_a_real_parameter", Value::Number(1.0)),
            Err(OperationError::UnknownParameter(_))
        ));
    }
}
