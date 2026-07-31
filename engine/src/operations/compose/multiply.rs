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
use crate::graphics::{Image, ImageFormat};

/// Multiply operation - multiplies RGBA channels from two inputs pixel by pixel.
pub struct Multiply;

impl Multiply {
    pub fn new() -> Self {
        Self
    }

    /// Multiply two RGBA pixel buffers channel by channel.
    /// Values are normalized back to 0-255.
    pub fn multiply_pixels(a: &[u8], b: &[u8]) -> Vec<u8> {
        let mut output = vec![0u8; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            target[0] = ((source_a[0] as u16 * source_b[0] as u16) / 255) as u8;
            target[1] = ((source_a[1] as u16 * source_b[1] as u16) / 255) as u8;
            target[2] = ((source_a[2] as u16 * source_b[2] as u16) / 255) as u8;
            target[3] = ((source_a[3] as u16 * source_b[3] as u16) / 255) as u8;
        }

        output
    }

    fn image_from_value(
        value: &Value,
        ctx: &Context,
    ) -> Result<Arc<Image>, OperationError> {
        match value {
            Value::Image(image) => Ok(image.clone()),

            Value::Frame(frame) => Ok(Arc::new(Image {
                pixels: frame.pixels.clone(),
                width: frame.width,
                height: frame.height,
                format: ImageFormat::Rgba8,
            })),

            Value::Video(video) => {
                Ok(video.frame_at(ctx.meta.time)?)
            }

            other => Err(OperationError::InvalidInputType(
                format!("Multiply cannot read {:?}", other)
            )),
        }
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
            outputs: vec![OutputKind::Image],
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

        let first_image = Self::image_from_value(first, ctx)?;
        let second_image = Self::image_from_value(second, ctx)?;

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
            Value::Image(Arc::new(Image {
                pixels: multiplied,
                width: first_image.width,
                height: first_image.height,
                format: first_image.format,
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

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<Image> {
        Arc::new(Image {
            pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        })
    }

    #[test]
    fn multiplying_by_white_is_identity() {
        let white = image(vec![255, 255, 255, 255], 1, 1);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Multiply::multiply_pixels(&white.pixels, &color.pixels);

        assert_eq!(out, color.pixels);
    }

    #[test]
    fn multiplying_by_black_is_black() {
        let black = image(vec![0, 0, 0, 255], 1, 1);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Multiply::multiply_pixels(&black.pixels, &color.pixels);

        assert_eq!(out, vec![0, 0, 0, 200]);
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

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, vec![10, 20, 30, 255]),
            other => panic!("expected image, got {:?}", other),
        }
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

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, fg.pixels),
            other => panic!("expected image, got {:?}", other),
        }
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
