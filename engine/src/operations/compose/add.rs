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
use crate::graphics::{FloatImage, Image, ImageFormat};

/// Add operation - adds RGBA channels from two inputs pixel by pixel,
/// unclamped: a sum above 1.0 is a legitimate out-of-gamut result (an
/// overexposed highlight, e.g.), same as any real compositor's Add/Plus
/// node - not an error to clip away here. CLAMP is the explicit,
/// deliberate step back down to a normal 0..1 Image. Same shape as
/// Multiply, other than that.
pub struct Add;

impl Add {
    pub fn new() -> Self {
        Self
    }

    /// Raw per-channel sum, normalized to 0.0..1.0 input range - NOT
    /// clamped. See this module's own doc comment for why.
    pub fn add_pixels(a: &[u8], b: &[u8]) -> Vec<f32> {
        let mut output = vec![0f32; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            for channel in 0..4 {
                target[channel] =
                    source_a[channel] as f32 / 255.0 + source_b[channel] as f32 / 255.0;
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
            display_name: "Add",
            category: OperationCategory::Color,
            // Identity (MASK=0) is Foreground unmodified - it's the input
            // that still makes sense to show on its own with no compositing
            // applied, unlike Background which is meaningless alone here.
            inputs: vec![Input::Foreground, Input::Background, Input::Mask],
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
        let added = crate::graphics::apply_mask_wide(&first_image.pixels, added, mask.as_ref(), first_image.width, first_image.height)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: added,
            width: first_image.width,
            height: first_image.height,
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
    fn adding_above_255_is_left_out_of_gamut_not_clamped() {
        let a = image(vec![200, 0, 100, 255], 1, 1);
        let b = image(vec![100, 50, 200, 255], 1, 1);

        let out = Add::add_pixels(&a.pixels, &b.pixels);

        // 200+100 and 100+200 both exceed 255 - left as real out-of-range
        // floats (300/255 ~= 1.176), not clipped to 1.0 here.
        assert!((out[0] - 300.0 / 255.0).abs() < 0.001);
        assert!((out[1] - 50.0 / 255.0).abs() < 0.001);
        assert!((out[2] - 300.0 / 255.0).abs() < 0.001);
    }

    #[test]
    fn a_sum_that_stays_in_gamut_round_trips_through_clamp_unchanged() {
        // The common case (no overflow) should still recover the exact
        // same u8 result CLAMP would produce, once the caller clamps it.
        let a = image(vec![100, 0, 50, 255], 1, 1);
        let b = image(vec![50, 20, 30, 0], 1, 1);

        let out = Add::add_pixels(&a.pixels, &b.pixels);
        let float_image = FloatImage { pixels: out, width: 1, height: 1 };
        let clamped = float_image.to_image_clamped(0.0, 1.0);

        assert_eq!(clamped.pixels, vec![150, 20, 80, 255]);
    }

    #[test]
    fn adding_black_is_identity() {
        let black = image(vec![0, 0, 0, 0], 1, 1);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Add::add_pixels(&black.pixels, &color.pixels);
        let expected = FloatImage::from_image(&color).pixels;

        for (a, b) in out.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 0.001);
        }
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
            Value::FloatImage(out) => {
                let expected = FloatImage::from_image(&fg).pixels;
                for (a, b) in out.pixels.iter().zip(expected.iter()) {
                    assert!((a - b).abs() < 0.001);
                }
            }
            other => panic!("expected a float image, got {:?}", other),
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
            Value::FloatImage(out) => {
                let expected = Add::add_pixels(&fg.pixels, &bg.pixels);
                for (a, b) in out.pixels.iter().zip(expected.iter()) {
                    assert!((a - b).abs() < 0.001);
                }
            }
            other => panic!("expected a float image, got {:?}", other),
        }
    }
}
