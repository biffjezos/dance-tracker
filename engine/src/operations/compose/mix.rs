// src/operations/compose/mix.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    input::find_input,
    Operation,
    OperationDescriptor,
    metadata::{OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind},
    Value,
};
use crate::graphics::FloatImage;

/// Crossfades two pixel sources by a single uniform AMOUNT - not a
/// revival of the "MIX vs generic MASK" decision (see
/// ANIMATION_IMPLEMENTATION_PLAN.md's MIX section): MASK modulates one
/// operation's own effect strength via another node's per-pixel alpha,
/// this blends two independent sources by one scalar. Exists as a
/// purpose-built, always-eligible target for animation wiring - unlike
/// most operations, MIX's AMOUNT is a Number parameter every instance
/// has, regardless of what the two blended sources are.
pub struct Mix {
    pub amount: f64,
}

impl Mix {
    pub fn new() -> Self {
        Self { amount: 0.5 }
    }

    /// Per-channel crossfade, all 4 channels uniformly (same convention
    /// as Add/Multiply/Screen) - NOT Ghost's alpha-aware Porter-Duff
    /// "over"; this is a plain lerp, not a compositing operator.
    pub fn mix_pixels(a: &[f32], b: &[f32], amount: f64) -> Vec<f32> {
        let amount = amount as f32;
        let mut output = vec![0f32; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            for channel in 0..4 {
                target[channel] = source_a[channel] * (1.0 - amount) + source_b[channel] * amount;
            }
        }

        output
    }
}

impl Default for Mix {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Mix {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "mix",
            menu: "COMPOSE",
            label: "MIX",
            action: None,
            ui_action: None,
            create_node: Some("mix"),
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
            display_name: "Mix",
            category: OperationCategory::Composite,
            inputs: vec![Input::Foreground, Input::Background],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "AMOUNT",
                kind: ParameterKind::Number { step: 0.01, min: Some(0.0), max: Some(1.0) },
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "AMOUNT" => Some(Value::Number(self.amount)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("AMOUNT", Value::Number(v)) => {
                if !(0.0..=1.0).contains(&v) {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.amount = v;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(first) = find_input(inputs, Input::Foreground) else {
            return Err(OperationError::InvalidInputType("Mix requires FOREGROUND".into()));
        };

        let Some(second) = find_input(inputs, Input::Background) else {
            return Err(OperationError::InvalidInputType("Mix requires BACKGROUND".into()));
        };

        let first_image = FloatImage::from_value(first, ctx)?;
        let second_image = FloatImage::from_value(second, ctx)?;

        if first_image.width != second_image.width || first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Mix inputs must have matching dimensions".into()
            ));
        }

        let mixed = Self::mix_pixels(&first_image.pixels, &second_image.pixels, self.amount);

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: mixed,
            width: first_image.width,
            height: first_image.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Mix::new())
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

    #[test]
    fn amount_zero_is_pure_foreground() {
        let a = FloatImage::from_image(&image(vec![10, 20, 30, 255], 1, 1)).pixels;
        let b = FloatImage::from_image(&image(vec![200, 210, 220, 255], 1, 1)).pixels;
        let out = Mix::mix_pixels(&a, &b, 0.0);
        assert_eq!(out, a);
    }

    #[test]
    fn amount_one_is_pure_background() {
        let a = FloatImage::from_image(&image(vec![10, 20, 30, 255], 1, 1)).pixels;
        let b = FloatImage::from_image(&image(vec![200, 210, 220, 255], 1, 1)).pixels;
        let out = Mix::mix_pixels(&a, &b, 1.0);
        assert_eq!(out, b);
    }

    #[test]
    fn amount_half_averages_both_inputs() {
        let a = vec![0.0, 0.0, 0.0, 0.0];
        let b = vec![1.0, 1.0, 1.0, 1.0];
        let out = Mix::mix_pixels(&a, &b, 0.5);
        for c in out {
            assert!((c - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn set_parameter_rejects_an_amount_above_one() {
        let mut mix = Mix::new();
        let err = mix.set_parameter("AMOUNT", Value::Number(1.5)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn set_parameter_rejects_a_negative_amount() {
        let mut mix = Mix::new();
        let err = mix.set_parameter("AMOUNT", Value::Number(-0.1)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn execute_errors_without_a_wired_foreground() {
        let mix = Mix::new();
        let bg = Value::Image(image(vec![0, 0, 0, 255], 1, 1));
        let err = mix.execute(&context(1, 1), &[(Input::Background, bg)]).unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn execute_errors_without_a_wired_background() {
        let mix = Mix::new();
        let fg = Value::Image(image(vec![0, 0, 0, 255], 1, 1));
        let err = mix.execute(&context(1, 1), &[(Input::Foreground, fg)]).unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn execute_errors_on_mismatched_dimensions() {
        let mix = Mix::new();
        let fg = Value::Image(image(vec![0, 0, 0, 255, 0, 0, 0, 255], 2, 1));
        let bg = Value::Image(image(vec![0, 0, 0, 255], 1, 1));
        let err = mix
            .execute(&context(2, 1), &[(Input::Foreground, fg), (Input::Background, bg)])
            .unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn mix_combines_two_wired_inputs_by_amount() {
        let mut mix = Mix::new();
        mix.amount = 0.25;

        let fg = Value::Image(image(vec![0, 0, 0, 0], 1, 1));
        let bg = Value::Image(image(vec![255, 255, 255, 255], 1, 1));

        let values = mix
            .execute(&context(1, 1), &[(Input::Foreground, fg), (Input::Background, bg)])
            .unwrap();

        match &values[0] {
            Value::FloatImage(out) => {
                assert!((out.pixels[0] - 0.25).abs() < 1e-6);
                assert!((out.pixels[3] - 0.25).abs() < 1e-6);
            }
            other => panic!("expected a float image, got {:?}", other),
        }
    }
}
