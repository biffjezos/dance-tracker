// src/operations/compose/subtract.rs
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

/// Subtract operation - Foreground minus Background, per channel,
/// unclamped: a difference below 0.0 is a legitimate out-of-gamut result
/// (used deliberately in matte/difference work), same as any real
/// compositor's Subtract/Minus node - not an error to clip away here.
/// CLAMP is the explicit, deliberate step back down to a normal 0..1
/// Image. Same shape as Multiply, other than that. Both inputs accept a
/// bounded Image or an already-unbounded FloatImage alike (via
/// FloatImage::from_value).
pub struct Subtract;

impl Subtract {
    pub fn new() -> Self {
        Self
    }

    /// Raw per-channel difference - NOT clamped. See this module's own
    /// doc comment for why.
    pub fn subtract_pixels(a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut output = vec![0f32; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            for channel in 0..4 {
                target[channel] = source_a[channel] - source_b[channel];
            }
        }

        output
    }
}

impl Default for Subtract {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Subtract {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "subtract",
            menu: "COMPOSE",
            label: "SUBTRACT",
            action: None,
            ui_action: None,
            create_node: Some("subtract"),
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
            display_name: "Subtract",
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
            return Err(OperationError::InvalidInputType("Subtract requires first input".into()));
        };

        let Some(second) = find_input(inputs, Input::Background) else {
            return Err(OperationError::InvalidInputType("Subtract requires second input".into()));
        };

        let first_image = FloatImage::from_value(first, ctx)?;
        let second_image = FloatImage::from_value(second, ctx)?;

        if first_image.width != second_image.width || first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Subtract inputs must have matching dimensions".into()
            ));
        }

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        let subtracted = Self::subtract_pixels(&first_image.pixels, &second_image.pixels);
        let subtracted = crate::graphics::apply_mask(&first_image.pixels, subtracted, mask.as_ref(), first_image.width, first_image.height)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: subtracted,
            width: first_image.width,
            height: first_image.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Subtract::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<crate::graphics::U8Image> {
        Arc::new(crate::graphics::U8Image { pixels, width, height, format: crate::graphics::ImageFormat::Rgba8 })
    }

    fn float_pixels(pixels: Vec<u8>) -> Vec<f32> {
        FloatImage::from_image(&image(pixels, 1, 1)).pixels
    }

    #[test]
    fn subtracting_below_zero_is_left_out_of_gamut_not_clamped() {
        let a = float_pixels(vec![50, 0, 100, 255]);
        let b = float_pixels(vec![100, 50, 30, 10]);

        let out = Subtract::subtract_pixels(&a, &b);

        // 50-100 and 0-50 both go negative - left as real out-of-range
        // floats (-50/255 ~= -0.196), not clipped to 0.0 here.
        assert!((out[0] - (-50.0 / 255.0)).abs() < 0.001);
        assert!((out[1] - (-50.0 / 255.0)).abs() < 0.001);
        assert!((out[2] - 70.0 / 255.0).abs() < 0.001);
    }

    #[test]
    fn a_difference_that_stays_in_gamut_round_trips_through_clamp_unchanged() {
        let a = float_pixels(vec![50, 0, 100, 255]);
        let b = float_pixels(vec![10, 0, 30, 10]);

        let out = Subtract::subtract_pixels(&a, &b);
        let float_image = FloatImage { pixels: out, width: 1, height: 1 };
        let clamped = float_image.to_image_clamped(0.0, 1.0);

        assert_eq!(clamped.pixels, vec![40, 0, 70, 245]);
    }

    #[test]
    fn subtracting_black_is_identity() {
        let black = float_pixels(vec![0, 0, 0, 0]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Subtract::subtract_pixels(&FloatImage::from_image(&color).pixels, &black);
        let expected = FloatImage::from_image(&color).pixels;

        for (a, b) in out.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }

    #[test]
    fn chaining_subtract_into_subtract_accepts_the_out_of_gamut_float_image_input() {
        // Regression: SUBTRACT used to only accept a bounded
        // Image/Frame/Video, so wiring one SUBTRACT's output into another
        // errored out entirely.
        let inner = Subtract::subtract_pixels(&float_pixels(vec![0, 0, 0, 255]), &float_pixels(vec![200, 0, 0, 255]));
        let outer = Subtract::subtract_pixels(&inner, &float_pixels(vec![200, 0, 0, 255]));
        assert!(outer[0] < -1.0, "expected a further-negative out-of-gamut result, got {}", outer[0]);
    }

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn a_zero_alpha_mask_passes_through_foreground_unsubtracted() {
        let subtract = Subtract::new();
        let fg = image(vec![100, 100, 100, 255], 1, 1);
        let bg = image(vec![10, 20, 30, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 0], 1, 1);

        let values = subtract
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
}
