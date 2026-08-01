use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    input::find_input,
    Operation,
    OperationDescriptor,
    metadata::{ OperationCategory, OperationMetadata, OutputKind },
    Value,
};
use crate::graphics::FloatImage;

/// Multiply operation - multiplies RGBA channels from two inputs pixel by
/// pixel, unclamped: given two in-gamut (0.0..1.0) inputs the result can
/// never exceed either one, but multiplying an already out-of-gamut value
/// (e.g. 1.5 from an ADD result) correctly stays out of gamut (1.5 * 1.5 =
/// 2.25), not silently reclamped mid-calculation. Both inputs accept a
/// bounded Image or an already-unbounded FloatImage alike (via
/// FloatImage::from_value), so chaining another compose op's output
/// straight into Multiply works without an intervening CLAMP.
pub struct Multiply;

impl Multiply {
    pub fn new() -> Self {
        Self
    }

    /// Multiply two RGBA pixel buffers channel by channel - NOT clamped.
    /// See this module's own doc comment for why.
    pub fn multiply_pixels(a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut output = vec![0f32; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            target[0] = source_a[0] * source_b[0];
            target[1] = source_a[1] * source_b[1];
            target[2] = source_a[2] * source_b[2];
            target[3] = source_a[3] * source_b[3];
        }

        output
    }
}

impl Operation for Multiply {

    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "multiply",
            menu: "COMPOSE",
            label: "MULTIPLY",
            action: None,
            ui_action: None,
            create_node: Some("multiply"),
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
            display_name: "Multiply",
            category: OperationCategory::Color,
            // Identity (MASK=0) is Foreground unmodified - see add.rs's
            // metadata() for why Foreground and not Background.
            inputs: vec![
                 Input::Foreground,
                 Input::Background,
                 Input::Mask
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

    fn set_parameter(
        &mut self,
        name: &str,
        _value: Value,
    ) -> Result<(), OperationError> {
        Err(OperationError::UnknownParameter(name.to_string()))
    }

    fn execute(
        &self,
        ctx: &Context,
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {

        let Some(first) = find_input(inputs, Input::Foreground) else {
            return Err(OperationError::InvalidInputType(
                "Multiply requires first input".into()
            ));
        };

        let Some(second) = find_input(inputs, Input::Background) else {
            return Err(OperationError::InvalidInputType(
                "Multiply requires second input".into()
            ));
        };

        let first_image = FloatImage::from_value(first, ctx)?;
        let second_image = FloatImage::from_value(second, ctx)?;

        if first_image.width != second_image.width ||
           first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Multiply inputs must have matching dimensions".into()
            ));
        }

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        let multiplied = Self::multiply_pixels(
            &first_image.pixels,
            &second_image.pixels,
        );
        let multiplied = crate::graphics::apply_mask(
            &first_image.pixels,
            multiplied,
            mask.as_ref(),
            first_image.width,
            first_image.height,
        )?;

        Ok(vec![
            Value::FloatImage(Arc::new(FloatImage {
                pixels: multiplied,
                width: first_image.width,
                height: first_image.height,
            }))
        ])
    }
}

// Inventory registration for Multiply
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Multiply::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta {
                width,
                height,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<crate::graphics::U8Image> {
        Arc::new(crate::graphics::U8Image {
            pixels,
            width,
            height,
            format: crate::graphics::ImageFormat::Rgba8,
        })
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
    fn multiplying_by_white_is_identity() {
        let white = float_pixels(vec![255, 255, 255, 255]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Multiply::multiply_pixels(&white, &FloatImage::from_image(&color).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);

        assert_eq!(out.pixels, color.pixels);
    }

    #[test]
    fn multiplying_by_black_is_black() {
        let black = float_pixels(vec![0, 0, 0, 255]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Multiply::multiply_pixels(&black, &FloatImage::from_image(&color).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);

        assert_eq!(out.pixels, vec![0, 0, 0, 200]);
    }

    #[test]
    fn multiplying_two_out_of_gamut_values_stays_out_of_gamut() {
        // Regression: MULTIPLY used to only accept a bounded
        // Image/Frame/Video and clamp inline via u16/255 math - both
        // of which broke once a compose op's own output (FloatImage)
        // could be wired straight into it.
        let a = vec![1.5f32, 1.5, 1.5, 1.0];
        let b = vec![1.5f32, 1.5, 1.5, 1.0];

        let out = Multiply::multiply_pixels(&a, &b);

        assert!((out[0] - 2.25).abs() < 0.001);
    }

    #[test]
    fn multiply_in_graph_requires_both_inputs_of_matching_size() {
        let multiply = Multiply::new();

        let a = Value::Image(image(vec![255, 0, 0, 255], 1, 1));
        let b = Value::Image(image(vec![0, 0, 0, 255, 0, 0, 0, 255], 2, 1));

        let err = multiply
            .execute(&context(1, 1), &[(Input::Foreground, a), (Input::Background, b)])
            .unwrap_err();

        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn multiply_combines_two_wired_inputs() {
        let multiply = Multiply::new();

        let fg = Value::Image(image(vec![255, 255, 255, 255], 1, 1));
        let bg = Value::Image(image(vec![10, 20, 30, 255], 1, 1));

        let values = multiply
            .execute(&context(1, 1), &[(Input::Foreground, fg), (Input::Background, bg)])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), vec![10, 20, 30, 255]);
    }

    #[test]
    fn a_zero_alpha_mask_passes_through_foreground_unmultiplied() {
        let multiply = Multiply::new();
        let fg = image(vec![10, 20, 30, 255], 1, 1);
        let bg = image(vec![0, 0, 0, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 0], 1, 1);

        let values = multiply
            .execute(&context(1, 1), &[
                (Input::Foreground, Value::Image(fg.clone())),
                (Input::Background, Value::Image(bg)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), fg.pixels);
    }

    #[test]
    fn a_mismatched_mask_size_errors_instead_of_being_silently_ignored() {
        let multiply = Multiply::new();
        let fg = image(vec![10, 20, 30, 255], 1, 1);
        let bg = image(vec![0, 0, 0, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 255, 0, 0, 0, 255], 2, 1);

        let err = multiply
            .execute(&context(1, 1), &[
                (Input::Foreground, Value::Image(fg)),
                (Input::Background, Value::Image(bg)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap_err();

        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }
}
