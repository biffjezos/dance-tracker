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
    metadata::{OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor},
    Value,
};
use crate::graphics::{Frame, Image};

/// Inverts every channel per pixel (1 - value), alpha included - matching
/// Multiply's convention of treating all 4 channels uniformly, which keeps
/// the blend-mode algebra exact (Screen(A,B) = Invert(Multiply(Invert(A),
/// Invert(B)))). Useful on its own, and as a building block for other blend
/// modes.
pub struct Invert;

impl Invert {
    pub fn new() -> Self {
        Self
    }

    pub fn invert_pixels(pixels: &[u8]) -> Vec<u8> {
        pixels.iter().map(|channel| 255 - channel).collect()
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
            inputs: vec![Input::Source, Input::Mask],
            outputs: vec![OutputKind::Image],
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
            return Ok(vec![Value::Image(Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        match value {
            Value::Frame(frame) => {
                let inverted = Self::invert_pixels(&frame.pixels);
                let inverted = crate::graphics::apply_mask(&frame.pixels, inverted, mask.as_ref(), frame.width, frame.height)?;
                Ok(vec![Value::Frame(Arc::new(Frame {
                    pixels: inverted,
                    width: frame.width,
                    height: frame.height,
                    timestamp: frame.timestamp,
                }))])
            }

            Value::Image(image) => {
                let inverted = Self::invert_pixels(&image.pixels);
                let inverted = crate::graphics::apply_mask(&image.pixels, inverted, mask.as_ref(), image.width, image.height)?;
                Ok(vec![Value::Image(Arc::new(Image {
                    pixels: inverted,
                    width: image.width,
                    height: image.height,
                    format: image.format,
                }))])
            }

            Value::Video(video) => {
                let image = video.frame_at(ctx.meta.time)?;
                let inverted = Self::invert_pixels(&image.pixels);
                let inverted = crate::graphics::apply_mask(&image.pixels, inverted, mask.as_ref(), image.width, image.height)?;
                Ok(vec![Value::Image(Arc::new(Image {
                    pixels: inverted,
                    width: image.width,
                    height: image.height,
                    format: image.format,
                }))])
            }

            other => Err(OperationError::InvalidInputType(format!(
                "Invert cannot process {:?}",
                other
            ))),
        }
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
    use crate::graphics::ImageFormat;

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

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<Image> {
        Arc::new(Image {
            pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        })
    }

    #[test]
    fn inverting_black_is_white() {
        let black = image(vec![0, 0, 0, 128], 1, 1);
        let out = Invert::invert_pixels(&black.pixels);
        assert_eq!(out, vec![255, 255, 255, 127]);
    }

    #[test]
    fn inverting_twice_is_identity() {
        let color = image(vec![10, 200, 50, 255], 1, 1);
        let once = Invert::invert_pixels(&color.pixels);
        let twice = Invert::invert_pixels(&once);
        assert_eq!(twice, color.pixels);
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

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, input.pixels),
            other => panic!("expected image, got {:?}", other),
        }
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

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, vec![245, 55, 205, 0]),
            other => panic!("expected image, got {:?}", other),
        }
    }
}
