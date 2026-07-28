use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::Arc;

use crate::compositor::{
    find_input, Context, Input, Operation, OperationCategory, OperationError, OperationMetadata,
    OutputKind, ParameterDescriptor, ParameterKind, Value,
};
use crate::operations::composite::BlendMode;
use crate::operations::{expect_frame_arc, Frame};

/*
Input::Source is whatever's trailing (usually a rings/shape node's
output, but any frame works). Captures a copy every delay_ticks calls,
keeps up to count of them, and screen-blends them together with fading
weight, newest strongest - same idea as the old JS Ghost.history, but
counting executor ticks rather than wall-clock milliseconds (Context
doesn't carry a clock; this keeps Ghost pure Rust and natively
testable instead of needing a wasm32-only timestamp source). A slightly
different "how often" than the old delay setting, same "fading trail"
result. History holds Arc<Frame> - capturing a tick is a refcount bump
against whatever the source node already produced, never a pixel copy.
*/
pub struct Ghost {
    pub count: usize,
    pub alpha: f32,
    pub delay_ticks: u32,
    history: RefCell<VecDeque<Arc<Frame>>>,
    ticks_since_capture: Cell<u32>,
}

impl Ghost {
    pub fn new(count: usize, alpha: f32, delay_ticks: u32) -> Self {
        Ghost {
            count,
            alpha,
            delay_ticks,
            history: RefCell::new(VecDeque::new()),
            ticks_since_capture: Cell::new(0),
        }
    }
}

impl Operation for Ghost {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Ghost",
            category: OperationCategory::Generator,
            input_count: 1,
            outputs: vec![OutputKind::Frame],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor { name: "count", kind: ParameterKind::Number },
            ParameterDescriptor { name: "alpha", kind: ParameterKind::Number },
            ParameterDescriptor { name: "delay_ticks", kind: ParameterKind::Number },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "count" => Some(Value::Number(self.count as f64)),
            "alpha" => Some(Value::Number(self.alpha as f64)),
            "delay_ticks" => Some(Value::Number(self.delay_ticks as f64)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("count", Value::Number(v)) => { self.count = v.max(0.0) as usize; Ok(()) }
            ("alpha", Value::Number(v)) => { self.alpha = v as f32; Ok(()) }
            ("delay_ticks", Value::Number(v)) => { self.delay_ticks = v.max(0.0) as u32; Ok(()) }
            ("count" | "alpha" | "delay_ticks", _) => Err(OperationError::WrongValueType),
            _ => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    fn execute(
        &self,
        _ctx: &Context,
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        let source = expect_frame_arc(find_input(inputs, Input::Source))?;

        let ticks = self.ticks_since_capture.get() + 1;

        if ticks >= self.delay_ticks.max(1) {
            let mut history = self.history.borrow_mut();

            history.push_front(source.clone());

            while history.len() > self.count.max(1) {
                history.pop_back();
            }

            self.ticks_since_capture.set(0);
        } else {
            self.ticks_since_capture.set(ticks);
        }

        let history = self.history.borrow();
        let len = history.len();

        let mut result = Frame::blank(source.width, source.height, source.timestamp);

        for (i, past) in history.iter().enumerate() {
            let weight = self.alpha * (1.0 - (i as f32 / len.max(1) as f32));

            screen_blend_into(&mut result, past, weight);
        }

        Ok(vec![Value::Frame(Arc::new(result))])
    }
}

fn screen_blend_into(dst: &mut Frame, src: &Frame, weight: f32) {
    if !dst.same_dimensions(src) {
        return;
    }

    for i in (0..dst.pixels.len()).step_by(4) {
        for c in 0..3 {
            let blended = BlendMode::Screen.blend_channel(src.pixels[i + c], dst.pixels[i + c]);

            let mixed = dst.pixels[i + c] as f32 * (1.0 - weight) + blended as f32 * weight;

            dst.pixels[i + c] = mixed.round().clamp(0.0, 255.0) as u8;
        }

        let added_alpha = dst.pixels[i + 3] as f32 + weight * src.pixels[i + 3] as f32;

        dst.pixels[i + 3] = added_alpha.round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(pixels: Vec<u8>) -> Frame {
        Frame { pixels, width: 1, height: 1, timestamp: 0.0 }
    }

    fn tick(op: &Ghost, source: &Frame) -> Frame {
        let ctx = Context::default();
        let inputs = vec![(Input::Source, Value::Frame(Arc::new(source.clone())))];

        let mut outputs = op.execute(&ctx, &inputs).expect("should succeed");

        match outputs.remove(0) {
            Value::Frame(frame) => (*frame).clone(),
            _ => panic!("should be a Frame"),
        }
    }

    #[test]
    fn no_history_yet_produces_a_blank_frame() {
        let op = Ghost::new(4, 0.45, 3);
        let source = frame(vec![255, 255, 255, 255]);

        let out = tick(&op, &source);

        assert_eq!(out.pixels, vec![0, 0, 0, 0]);
    }

    #[test]
    fn once_delay_ticks_pass_the_trail_shows_non_zero_content() {
        let op = Ghost::new(4, 0.45, 1);
        let source = frame(vec![255, 255, 255, 255]);

        let out = tick(&op, &source);

        assert!(out.pixels[3] > 0, "expected some accumulated alpha, got {:?}", out.pixels);
    }

    #[test]
    fn history_never_grows_past_count() {
        let op = Ghost::new(2, 0.45, 1);
        let source = frame(vec![10, 10, 10, 255]);

        for _ in 0..10 {
            tick(&op, &source);
        }

        assert_eq!(op.history.borrow().len(), 2);
    }

    #[test]
    fn parameters_round_trip_and_take_effect() {
        let mut op = Ghost::new(4, 0.45, 3);

        assert!(matches!(op.get_parameter("count"), Some(Value::Number(v)) if v == 4.0));
        assert!(matches!(op.get_parameter("alpha"), Some(Value::Number(v)) if (v - 0.45).abs() < 1e-6));
        assert!(matches!(op.get_parameter("delay_ticks"), Some(Value::Number(v)) if v == 3.0));

        op.set_parameter("count", Value::Number(2.0)).expect("should accept a Number");
        op.set_parameter("delay_ticks", Value::Number(1.0)).expect("should accept a Number");

        assert_eq!(op.count, 2);
        assert_eq!(op.delay_ticks, 1);

        // count now caps history at 2 instead of 4, delay_ticks captures
        // every tick instead of every 3rd - both take effect immediately,
        // no rebuild required.
        let source = frame(vec![10, 10, 10, 255]);
        for _ in 0..10 {
            tick(&op, &source);
        }
        assert_eq!(op.history.borrow().len(), 2);
    }

    #[test]
    fn set_parameter_rejects_wrong_type_and_unknown_name() {
        let mut op = Ghost::new(4, 0.45, 3);

        assert!(matches!(
            op.set_parameter("count", Value::Text("nope".to_string())),
            Err(OperationError::WrongValueType)
        ));

        assert!(matches!(
            op.set_parameter("not_a_real_parameter", Value::Number(1.0)),
            Err(OperationError::UnknownParameter(_))
        ));
    }
}
