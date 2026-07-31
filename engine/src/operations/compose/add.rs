// src/operations/compose/add.rs
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

/// Add operation - adds RGBA channels from two inputs pixel by pixel,
/// clamped to 255. Simple building block, same shape as Multiply.
pub struct Add;

impl Add {
    pub fn new() -> Self {
        Self
    }

    pub fn add_pixels(a: &[u8], b: &[u8]) -> Vec<u8> {
        let mut output = vec![0u8; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            for channel in 0..4 {
                target[channel] = (source_a[channel] as u16 + source_b[channel] as u16)
                    .min(255) as u8;
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
                format!("Add cannot read {:?}", other)
            )),
        }
    }
}

impl Default for Add {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Add {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "add",
            menu: "COMPOSE",
            label: "ADD",
            action: None,
            ui_action: None,
            create_node: Some("add"),
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
            display_name: "Add",
            category: OperationCategory::Color,
            // Identity (MASK=0) is Foreground unmodified - it's the input
            // that still makes sense to show on its own with no compositing
            // applied, unlike Background which is meaningless alone here.
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
            return Err(OperationError::InvalidInputType("Add requires first input".into()));
        };

        let Some(second) = find_input(inputs, Input::Background) else {
            return Err(OperationError::InvalidInputType("Add requires second input".into()));
        };

        let first_image = Self::image_from_value(first, ctx)?;
        let second_image = Self::image_from_value(second, ctx)?;

        if first_image.width != second_image.width || first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Add inputs must have matching dimensions".into()
            ));
        }

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        let added = Self::add_pixels(&first_image.pixels, &second_image.pixels);
        let added = crate::graphics::apply_mask(&first_image.pixels, added, mask.as_ref(), first_image.width, first_image.height)?;

        Ok(vec![Value::Image(Arc::new(Image {
            pixels: added,
            width: first_image.width,
            height: first_image.height,
            format: first_image.format,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Add::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<Image> {
        Arc::new(Image { pixels, width, height, format: ImageFormat::Rgba8 })
    }

    #[test]
    fn adding_clamps_at_255() {
        let a = image(vec![200, 0, 100, 255], 1, 1);
        let b = image(vec![100, 50, 200, 255], 1, 1);

        let out = Add::add_pixels(&a.pixels, &b.pixels);

        assert_eq!(out, vec![255, 50, 255, 255]);
    }

    #[test]
    fn adding_black_is_identity() {
        let black = image(vec![0, 0, 0, 0], 1, 1);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Add::add_pixels(&black.pixels, &color.pixels);

        assert_eq!(out, color.pixels);
    }

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn a_zero_alpha_mask_passes_through_foreground_unadded() {
        let add = Add::new();
        let fg = image(vec![10, 20, 30, 255], 1, 1);
        let bg = image(vec![100, 100, 100, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 0], 1, 1);

        let values = add
            .execute(&context(1, 1), &[
                (Input::Foreground, Value::Image(fg.clone())),
                (Input::Background, Value::Image(bg)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, fg.pixels),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn a_full_alpha_mask_adds_exactly_as_unmasked() {
        let add = Add::new();
        let fg = image(vec![10, 20, 30, 255], 1, 1);
        let bg = image(vec![100, 100, 100, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 255], 1, 1);

        let values = add
            .execute(&context(1, 1), &[
                (Input::Foreground, Value::Image(fg.clone())),
                (Input::Background, Value::Image(bg.clone())),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, Add::add_pixels(&fg.pixels, &bg.pixels)),
            other => panic!("expected an image, got {:?}", other),
        }
    }
}
