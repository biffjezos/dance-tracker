use std::sync::Arc;

use crate::compositor::{
    find_input, Context, Input, Operation, OperationCategory, OperationError, OperationMetadata,
    OutputKind, Value,
};
use crate::operations::{expect_frame, Frame};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Red,
    Green,
    Blue,
    Alpha,
}

impl Channel {
    fn value(&self, pixels: &[u8], i: usize) -> u8 {
        match self {
            Channel::Red => pixels[i],
            Channel::Green => pixels[i + 1],
            Channel::Blue => pixels[i + 2],
            Channel::Alpha => pixels[i + 3],
        }
    }
}

/*
Input::Content is the content being masked, Input::Mask is the mask
source - content's own RGB is untouched, only its alpha is scaled by
the mask source's selected channel value (0 -> fully transparent
there, 255 -> unchanged). This IS "MASKED BY": wiring another node in
as Input::Mask here is the whole feature, not a settings field on the
masked node. Ports the old renderer.js applyMask/maskChannelValue math.
*/
pub struct ApplyMask {
    pub channel: Channel,
}

impl Operation for ApplyMask {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Apply Mask",
            category: OperationCategory::Composite,
            input_count: 2,
            outputs: vec![OutputKind::Frame],
        }
    }

    fn execute(
        &self,
        _ctx: &Context,
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        let content = expect_frame(find_input(inputs, Input::Content))?;
        let mask = expect_frame(find_input(inputs, Input::Mask))?;

        if !content.same_dimensions(mask) {
            return Err(OperationError::DimensionMismatch);
        }

        let mut pixels = content.pixels.clone();

        for i in (0..pixels.len()).step_by(4) {
            let mask_value = self.channel.value(&mask.pixels, i);

            pixels[i + 3] =
                ((pixels[i + 3] as u32 * mask_value as u32) / 255) as u8;
        }

        let frame = Frame {
            pixels,
            width: content.width,
            height: content.height,
            timestamp: content.timestamp,
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

    fn run(channel: Channel, content: Frame, mask: Frame) -> Result<Frame, OperationError> {
        let op = ApplyMask { channel };
        let ctx = Context::default();
        let inputs = vec![
            (Input::Content, Value::Frame(Arc::new(content))),
            (Input::Mask, Value::Frame(Arc::new(mask))),
        ];

        let mut outputs = op.execute(&ctx, &inputs)?;

        match outputs.remove(0) {
            Value::Frame(frame) => Ok((*frame).clone()),
            _ => panic!("should be a Frame"),
        }
    }

    #[test]
    fn full_alpha_channel_leaves_content_unchanged() {
        let content = frame(vec![10, 20, 30, 200]);
        let mask = frame(vec![0, 0, 0, 255]);

        let out = run(Channel::Alpha, content, mask).unwrap();

        assert_eq!(out.pixels, vec![10, 20, 30, 200]);
    }

    #[test]
    fn zero_alpha_channel_makes_content_fully_transparent() {
        let content = frame(vec![10, 20, 30, 200]);
        let mask = frame(vec![0, 0, 0, 0]);

        let out = run(Channel::Alpha, content, mask).unwrap();

        assert_eq!(out.pixels, vec![10, 20, 30, 0]);
    }

    #[test]
    fn red_channel_of_mask_scales_alpha_not_rgb() {
        let content = frame(vec![10, 20, 30, 255]);
        let mask = frame(vec![128, 0, 0, 0]);

        let out = run(Channel::Red, content, mask).unwrap();

        assert_eq!(out.pixels, vec![10, 20, 30, 128]);
    }

    #[test]
    fn mismatched_dimensions_error_instead_of_panicking() {
        let content = frame(vec![10, 20, 30, 255]);
        let mask = Frame {
            pixels: vec![0, 0, 0, 255, 0, 0, 0, 255],
            width: 2,
            height: 1,
            timestamp: 0.0,
        };

        let result = run(Channel::Alpha, content, mask);

        assert!(matches!(result, Err(OperationError::DimensionMismatch)));
    }
}
