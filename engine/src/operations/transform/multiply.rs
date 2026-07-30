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
use crate::graphics::{Frame, Image};

/// Multiply operation - multiplies channels from two inputs pixel by pixel.
pub struct Multiply;

impl Multiply {
    pub fn new() -> Self {
        Self
    }

    fn multiply_pixels(a: &[u8], b: &[u8]) -> Vec<u8> {
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
}

impl Operation for Multiply {

    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "multiply",
            menu: "TRANSFORM",
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
            inputs: vec![Input::Source, Input::Source2],
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

        let Some(first) = find_input(inputs, Input::Source) else {
            return Err(OperationError::InvalidInputType(
                "Multiply requires first input".into()
            ));
        };

        let Some(second) = find_input(inputs, Input::Source2) else {
            return Err(OperationError::InvalidInputType(
                "Multiply requires second input".into()
            ));
        };

        let first_image = match first {
            Value::Image(image) => image.clone(),
            Value::Frame(frame) => Arc::new(Image {
                pixels: frame.pixels.clone(),
                width: frame.width,
                height: frame.height,
                format: crate::graphics::ImageFormat::Rgba8,
            }),
            Value::Video(video) => video.frame_at(ctx.meta.time)?,
            other => {
                return Err(OperationError::InvalidInputType(
                    format!("Multiply cannot read {:?}", other)
                ))
            }
        };

        let second_image = match second {
            Value::Image(image) => image.clone(),
            Value::Frame(frame) => Arc::new(Image {
                pixels: frame.pixels.clone(),
                width: frame.width,
                height: frame.height,
                format: crate::graphics::ImageFormat::Rgba8,
            }),
            Value::Video(video) => video.frame_at(ctx.meta.time)?,
            other => {
                return Err(OperationError::InvalidInputType(
                    format!("Multiply cannot read {:?}", other)
                ))
            }
        };

        if first_image.width != second_image.width ||
           first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Multiply inputs must have matching dimensions".into()
            ));
        }

        Ok(vec![Value::Image(Arc::new(Image {
            pixels: Self::multiply_pixels(
                &first_image.pixels,
                &second_image.pixels,
            ),
            width: first_image.width,
            height: first_image.height,
            format: first_image.format,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Multiply::new())
    }
}
