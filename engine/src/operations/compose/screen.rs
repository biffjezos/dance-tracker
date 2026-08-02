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
    metadata::{InputDescriptor, OperationCategory, OperationMetadata, OutputKind, PIXEL_KINDS},
    Value,
};
use crate::graphics::FloatImage;

/// Screen operation - the inverse of Multiply (Screen(A,B) =
/// Invert(Multiply(Invert(A), Invert(B)))), computed directly rather than
/// through three passes. Unclamped, same as Multiply/Add/Subtract - see
/// their own doc comments for why. Both inputs accept a bounded Image or
/// an already-unbounded FloatImage alike (via FloatImage::from_value).
pub struct Screen;

impl Screen {
    pub fn new() -> Self {
        Self
    }

    /// Screen two RGBA pixel buffers channel by channel - NOT clamped.
    pub fn screen_pixels(a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut output = vec![0f32; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            for channel in 0..4 {
                let inv_a = 1.0 - source_a[channel];
                let inv_b = 1.0 - source_b[channel];
                target[channel] = 1.0 - inv_a * inv_b;
            }
        }

        output
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
            submenu: None,
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
            inputs: vec![
                InputDescriptor { kind: Input::Foreground, accepts: PIXEL_KINDS },
                InputDescriptor { kind: Input::Background, accepts: PIXEL_KINDS },
                InputDescriptor { kind: Input::Mask, accepts: PIXEL_KINDS },
            ],
            outputs: vec![OutputKind::FloatImage],
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

        let first_image = FloatImage::from_value(first, ctx)?;
        let second_image = FloatImage::from_value(second, ctx)?;

        if first_image.width != second_image.width || first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Screen inputs must have matching dimensions".into()
            ));
        }

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        let screened = Self::screen_pixels(&first_image.pixels, &second_image.pixels);
        let screened = crate::graphics::apply_mask(&first_image.pixels, screened, mask.as_ref(), first_image.width, first_image.height)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: screened,
            width: first_image.width,
            height: first_image.height,
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

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<crate::graphics::U8Image> {
        Arc::new(crate::graphics::U8Image { pixels, width, height, format: crate::graphics::ImageFormat::Rgba8 })
    }

    fn float_pixels(pixels: Vec<u8>) -> Vec<f32> {
        FloatImage::from_image(&image(pixels, 1, 1)).pixels
    }

    fn as_u8_pixels(value: &Value) -> Vec<u8> {
        match value {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels,
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn screening_with_black_is_identity() {
        // All 4 channels are screened uniformly (matching Multiply's own
        // convention), so "black" here means every channel including
        // alpha is 0 - a channel left at 255 would screen as that
        // channel's own "white" case instead.
        let black = float_pixels(vec![0, 0, 0, 0]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Screen::screen_pixels(&black, &FloatImage::from_image(&color).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);

        assert_eq!(out.pixels, color.pixels);
    }

    #[test]
    fn screening_with_white_is_white() {
        let white = float_pixels(vec![255, 255, 255, 255]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Screen::screen_pixels(&white, &FloatImage::from_image(&color).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);

        assert_eq!(out.pixels, vec![255, 255, 255, 255]);
    }

    #[test]
    fn screen_is_invert_multiply_invert() {
        use crate::operations::compose::Multiply;

        let a = float_pixels(vec![80, 150, 30, 255]);
        let b = float_pixels(vec![200, 10, 90, 255]);

        let direct = Screen::screen_pixels(&a, &b);

        use crate::operations::transform::Invert;

        let inv_a = Invert::invert_pixels(&a);
        let inv_b = Invert::invert_pixels(&b);
        let multiplied = Multiply::multiply_pixels(&inv_a, &inv_b);
        let via_identity = Invert::invert_pixels(&multiplied);

        for (x, y) in direct.iter().zip(via_identity.iter()) {
            assert!((x - y).abs() < 0.001);
        }
    }

    #[test]
    fn screen_combines_two_wired_inputs() {
        let screen = Screen::new();

        let fg = Value::Image(image(vec![0, 0, 0, 255], 1, 1));
        let bg = Value::Image(image(vec![10, 20, 30, 255], 1, 1));

        let values = screen
            .execute(&context(1, 1), &[(Input::Foreground, fg), (Input::Background, bg)])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), vec![10, 20, 30, 255]);
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

        assert_eq!(as_u8_pixels(&values[0]), fg.pixels);
    }
}
