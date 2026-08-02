// src/operations/transform/invert.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    input::find_input,
    Operation,
    OperationDescriptor,
    metadata::{InputDescriptor, OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, PIXEL_KINDS},
    Value,
};
use crate::graphics::FloatImage;

/// Inverts every channel per pixel (1 - value), alpha included - matching
/// Multiply's convention of treating all 4 channels uniformly, which keeps
/// the blend-mode algebra exact (Screen(A,B) = Invert(Multiply(Invert(A),
/// Invert(B)))). Useful on its own, and as a building block for other blend
/// modes. Unclamped: inverting an out-of-gamut value (e.g. 1.5, from an
/// ADD result) correctly produces a negative one (-0.5), not 0.
pub struct Invert;

impl Invert {
    pub fn new() -> Self {
        Self
    }

    pub fn invert_pixels(pixels: &[f32]) -> Vec<f32> {
        pixels.iter().map(|channel| 1.0 - channel).collect()
    }
}

impl Default for Invert {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Invert {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "invert",
            menu: "TRANSFORM",
            label: "INVERT",
            action: None,
            ui_action: None,
            create_node: Some("invert"),
            submenu: Some("SPECTRA"),
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
            display_name: "Invert",
            category: OperationCategory::Color,
            inputs: vec![
                InputDescriptor { kind: Input::Source, accepts: PIXEL_KINDS },
                InputDescriptor { kind: Input::Mask, accepts: PIXEL_KINDS },
            ],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![]
    }

    fn get_parameter(&self, _name: &str) -> Option<Value> {
        None
    }

    fn set_parameter(&mut self, name: &str, _value: Value) -> Result<(), OperationError> {
        Err(OperationError::UnknownParameter(name.to_string()))
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        let source = FloatImage::from_value(value, ctx)?;

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        let inverted = Self::invert_pixels(&source.pixels);
        let inverted = crate::graphics::apply_mask(&source.pixels, inverted, mask.as_ref(), source.width, source.height)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: inverted,
            width: source.width,
            height: source.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Invert::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::{ImageFormat, U8Image};

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

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<U8Image> {
        Arc::new(U8Image {
            pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        })
    }

    fn as_u8_pixels(value: &Value) -> Vec<u8> {
        match value {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels,
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn inverting_black_is_white() {
        let black = image(vec![0, 0, 0, 128], 1, 1);
        let out = Invert::invert_pixels(&FloatImage::from_image(&black).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);
        assert_eq!(out.pixels, vec![255, 255, 255, 127]);
    }

    #[test]
    fn inverting_twice_is_identity() {
        let color = image(vec![10, 200, 50, 255], 1, 1);
        let start = FloatImage::from_image(&color).pixels;
        let once = Invert::invert_pixels(&start);
        let twice = Invert::invert_pixels(&once);
        let twice = FloatImage { pixels: twice, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);
        assert_eq!(twice.pixels, color.pixels);
    }

    #[test]
    fn inverting_an_out_of_gamut_value_goes_negative_not_to_zero() {
        // 1.5 inverted is -0.5, a real negative value - not clamped to 0
        // the way an 8-bit-only Invert would have to.
        let out = Invert::invert_pixels(&[1.5]);
        assert_eq!(out[0], -0.5);
    }

    #[test]
    fn unconnected_invert_produces_the_missing_placeholder() {
        let invert = Invert::new();
        let values = invert.execute(&context(2, 1), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => {
                assert_eq!(out.width, 2);
                assert_eq!(out.height, 1);
            }
            other => panic!("expected image, got {:?}", other),
        }
    }

    #[test]
    fn a_zero_alpha_mask_leaves_the_source_uninverted() {
        let invert = Invert::new();
        let input = image(vec![10, 200, 50, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 0], 1, 1);

        let values = invert
            .execute(&context(1, 1), &[
                (Input::Source, Value::Image(input.clone())),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), input.pixels);
    }

    #[test]
    fn a_full_alpha_mask_inverts_exactly_as_unmasked() {
        let invert = Invert::new();
        let input = image(vec![10, 200, 50, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 255], 1, 1);

        let values = invert
            .execute(&context(1, 1), &[
                (Input::Source, Value::Image(input)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), vec![245, 55, 205, 0]);
    }
}
