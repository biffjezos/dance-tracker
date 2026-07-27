use std::sync::Arc;

use crate::compositor::{
    find_input, Context, Input, Operation, OperationCategory, OperationError, OperationMetadata,
    OutputKind, ParameterDescriptor, ParameterKind, Value,
};
use crate::operations::masks::{key_pixel, Fill};
use crate::operations::{expect_frame, Frame};

/*
Input::Source is the video being keyed, Input::Reference is the
"empty room" frame - per-pixel, not a single colour like Chroma. This
operation itself is stateless; "CAPTURE BACKGROUND" in the UI is pure
node-graph rewiring (see sources::captured::CapturedFrame), not
something this operation does internally - wire a CapturedFrame node
into Input::Reference and repoint/recapture it whenever the user
clicks CAPTURE BACKGROUND. A live second feed works here too, for true
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

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Difference Key",
            category: OperationCategory::Mask,
            input_count: 2,
            outputs: vec![OutputKind::Frame],
        }
    }

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
        let reference = expect_frame(find_input(inputs, Input::Reference))?;

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
        let inputs = vec![
            (Input::Source, Value::Frame(Arc::new(video))),
            (Input::Reference, Value::Frame(Arc::new(reference))),
        ];

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
        let inputs = vec![
            (Input::Source, Value::Frame(Arc::new(video))),
            (Input::Reference, Value::Frame(Arc::new(reference))),
        ];

        let result = op.execute(&ctx, &inputs);

        assert!(matches!(result, Err(OperationError::DimensionMismatch)));
    }

    #[test]
    fn threshold_parameter_round_trips_and_takes_effect() {
        let mut op = Difference { threshold: 30, fill: Fill::Solid(255, 0, 255) };

        assert!(matches!(op.get_parameter("threshold"), Some(Value::Number(v)) if v == 30.0));

        op.set_parameter("threshold", Value::Number(2.0)).expect("should accept a Number");

        assert_eq!(op.threshold, 2);

        // |40-42| + |40-41| + |40-39| = 4, within the old threshold
        // (30) but past the lowered one (2), so it should now key as
        // differing instead of matching.
        let out = run(&op, frame(vec![40, 40, 40, 255]), frame(vec![42, 41, 39, 255]));
        assert_ne!(out.pixels, vec![0, 0, 0, 0], "lowered threshold should stop treating this as a match");
    }

    #[test]
    fn set_parameter_rejects_wrong_type_and_unknown_name() {
        let mut op = Difference { threshold: 30, fill: Fill::Solid(255, 0, 255) };

        assert!(matches!(
            op.set_parameter("threshold", Value::Boolean(true)),
            Err(OperationError::WrongValueType)
        ));

        assert!(matches!(
            op.set_parameter("not_a_real_parameter", Value::Number(1.0)),
            Err(OperationError::UnknownParameter(_))
        ));
    }
}
