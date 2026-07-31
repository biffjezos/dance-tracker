// src/operations/compose/screen.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    input::find_input,
    Operation,
    OperationDescriptor,
    metadata::{OperationCategory, OperationMetadata, OutputKind},
    Value,
};
use crate::graphics::{Image, ImageFormat};

/// Screen operation - the inverse of Multiply (Screen(A,B) =
/// Invert(Multiply(Invert(A), Invert(B)))), computed directly rather than
/// through three passes.
pub struct Screen;

impl Screen {
    pub fn new() -> Self {
        Self
    }

    /// Screen two RGBA pixel buffers channel by channel.
    pub fn screen_pixels(a: &[u8], b: &[u8]) -> Vec<u8> {
        let mut output = vec![0u8; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            for channel in 0..4 {
                let inv_a = 255 - source_a[channel] as u16;
                let inv_b = 255 - source_b[channel] as u16;
                target[channel] = 255 - ((inv_a * inv_b) / 255) as u8;
            }
        }

        output
    }

    fn image_from_value(value: &Value, ctx: &Context) -> Result<Arc<Image>, OperationError> {
        match value {
            Value::Image(image) => Ok(image.clone()),

            Value::Frame(frame) => Ok(Arc::new(Image {
                pixels: frame.pixels.clone(),
                width: frame.width,
                height: frame.height,
                format: ImageFormat::Rgba8,
            })),

            Value::Video(video) => Ok(video.frame_at(ctx.meta.time)?),

            other => Err(OperationError::InvalidInputType(
                format!("Screen cannot read {:?}", other)
            )),
        }
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Screen {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "screen",
            menu: "COMPOSE",
            label: "SCREEN",
            action: None,
            ui_action: None,
            create_node: Some("screen"),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Screen",
            category: OperationCategory::Color,
            // Identity (MASK=0) is Foreground unmodified - see add.rs's
            // metadata() for why Foreground and not Background.
            inputs: vec![Input::Foreground, Input::Background, Input::Mask],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<crate::compositor::metadata::ParameterDescriptor> {
        vec![]
    }

    fn get_parameter(&self, _name: &str) -> Option<Value> {
        None
    }

    fn set_parameter(&mut self, name: &str, _value: Value) -> Result<(), OperationError> {
        Err(OperationError::UnknownParameter(name.to_string()))
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(first) = find_input(inputs, Input::Foreground) else {
            return Err(OperationError::InvalidInputType("Screen requires first input".into()));
        };

        let Some(second) = find_input(inputs, Input::Background) else {
            return Err(OperationError::InvalidInputType("Screen requires second input".into()));
        };

        let first_image = Self::image_from_value(first, ctx)?;
        let second_image = Self::image_from_value(second, ctx)?;

        if first_image.width != second_image.width || first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Screen inputs must have matching dimensions".into()
            ));
        }

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_mask_pixels(v, ctx))
            .transpose()?;

        let screened = Self::screen_pixels(&first_image.pixels, &second_image.pixels);
        let screened = crate::graphics::apply_mask(&first_image.pixels, screened, mask.as_ref(), first_image.width, first_image.height)?;

        Ok(vec![Value::Image(Arc::new(Image {
            pixels: screened,
            width: first_image.width,
            height: first_image.height,
            format: first_image.format,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Screen::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<Image> {
        Arc::new(Image { pixels, width, height, format: ImageFormat::Rgba8 })
    }

    #[test]
    fn screening_with_black_is_identity() {
        // All 4 channels are screened uniformly (matching Multiply's own
        // convention), so "black" here means every channel including
        // alpha is 0 - a channel left at 255 would screen as that
        // channel's own "white" case instead.
        let black = image(vec![0, 0, 0, 0], 1, 1);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Screen::screen_pixels(&black.pixels, &color.pixels);

        assert_eq!(out, color.pixels);
    }

    #[test]
    fn screening_with_white_is_white() {
        let white = image(vec![255, 255, 255, 255], 1, 1);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Screen::screen_pixels(&white.pixels, &color.pixels);

        assert_eq!(out, vec![255, 255, 255, 255]);
    }

    #[test]
    fn screen_is_invert_multiply_invert() {
        use crate::operations::compose::Multiply;

        let a = image(vec![80, 150, 30, 255], 1, 1);
        let b = image(vec![200, 10, 90, 255], 1, 1);

        let direct = Screen::screen_pixels(&a.pixels, &b.pixels);

        use crate::operations::transform::Invert;

        let inv_a = Invert::invert_pixels(&a.pixels);
        let inv_b = Invert::invert_pixels(&b.pixels);
        let multiplied = Multiply::multiply_pixels(&inv_a, &inv_b);
        let via_identity = Invert::invert_pixels(&multiplied);

        assert_eq!(direct, via_identity);
    }

    #[test]
    fn screen_combines_two_wired_inputs() {
        let screen = Screen::new();

        let fg = Value::Image(image(vec![0, 0, 0, 255], 1, 1));
        let bg = Value::Image(image(vec![10, 20, 30, 255], 1, 1));

        let values = screen
            .execute(&context(1, 1), &[(Input::Foreground, fg), (Input::Background, bg)])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, vec![10, 20, 30, 255]),
            other => panic!("expected image, got {:?}", other),
        }
    }

    #[test]
    fn a_zero_alpha_mask_passes_through_foreground_unscreened() {
        let screen = Screen::new();
        let fg = image(vec![10, 20, 30, 255], 1, 1);
        let bg = image(vec![255, 255, 255, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 0], 1, 1);

        let values = screen
            .execute(&context(1, 1), &[
                (Input::Foreground, Value::Image(fg.clone())),
                (Input::Background, Value::Image(bg)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, fg.pixels),
            other => panic!("expected image, got {:?}", other),
        }
    }
}
