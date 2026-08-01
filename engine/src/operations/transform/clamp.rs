// src/operations/transform/clamp.rs
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

/// Explicit, deliberate step from an unbounded FloatImage back down to a
/// normal bounded Image - the one place an out-of-gamut value actually
/// gets thrown away. MIN/MAX default to 0.0/1.0 (the standard "bring back
/// into gamut" case) but are adjustable for a creative clip (crush
/// blacks, clip highlights early) - applied uniformly regardless of
/// whether the input happens to already be bounded, so a narrowed range
/// still does something to a normal Image/Frame/Video, not just a
/// FloatImage. With the default 0.0/1.0 range, clamping an
/// already-bounded input is a true no-op, which is what makes CLAMP
/// always safe to insert everywhere.
pub struct Clamp {
    pub min: f64,
    pub max: f64,
}

impl Clamp {
    pub fn new() -> Self {
        Self { min: 0.0, max: 1.0 }
    }
}

impl Default for Clamp {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Clamp {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "clamp",
            menu: "TRANSFORM",
            label: "CLAMP",
            action: None,
            ui_action: None,
            create_node: Some("clamp"),
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
            display_name: "Clamp",
            category: OperationCategory::Color,
            inputs: vec![Input::Source],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "MIN",
                kind: ParameterKind::Number { step: 0.01, min: Some(-10.0), max: Some(10.0) },
                group: None,
            },
            ParameterDescriptor {
                name: "MAX",
                kind: ParameterKind::Number { step: 0.01, min: Some(-10.0), max: Some(10.0) },
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "MIN" => Some(Value::Number(self.min)),
            "MAX" => Some(Value::Number(self.max)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("MIN", Value::Number(v)) => {
                if v >= self.max {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.min = v;
                Ok(())
            }
            ("MAX", Value::Number(v)) => {
                if v <= self.min {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.max = v;
                Ok(())
            }
            (name, _) => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        let source = FloatImage::from_value(value, ctx)?;
        let image = source.to_image_clamped(self.min as f32, self.max as f32);

        Ok(vec![Value::Image(Arc::new(image))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Clamp::new())
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

    #[test]
    fn clamps_an_out_of_gamut_float_image_to_the_default_0_1_range() {
        let clamp = Clamp::new();
        let float_image = FloatImage { pixels: vec![1.5, -0.2, 0.5, 1.0], width: 1, height: 1 };

        let values = clamp
            .execute(&context(1, 1), &[(Input::Source, Value::FloatImage(Arc::new(float_image)))])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, vec![255, 0, 128, 255]),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn a_custom_range_can_crush_or_extend_the_clip_points() {
        let mut clamp = Clamp::new();
        clamp.set_parameter("MIN", Value::Number(0.2)).unwrap();
        clamp.set_parameter("MAX", Value::Number(0.8)).unwrap();
        let float_image = FloatImage { pixels: vec![0.0, 1.0, 0.5, 1.0], width: 1, height: 1 };

        let values = clamp
            .execute(&context(1, 1), &[(Input::Source, Value::FloatImage(Arc::new(float_image)))])
            .unwrap();

        // Alpha is clamped uniformly with RGB (same convention as ADD/
        // SUBTRACT/MULTIPLY/INVERT treating all 4 channels the same way) -
        // the input's 1.0 alpha clips down to MAX (0.8) same as any
        // other channel would.
        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, vec![51, 204, 128, 204]),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn an_already_bounded_image_is_unchanged_by_the_default_range() {
        let clamp = Clamp::new();
        let image = Arc::new(crate::graphics::U8Image {
            pixels: vec![10, 20, 30, 255], width: 1, height: 1, format: crate::graphics::ImageFormat::Rgba8,
        });

        let values = clamp
            .execute(&context(1, 1), &[(Input::Source, Value::Image(image.clone()))])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, image.pixels),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn a_custom_range_crushes_an_already_bounded_image_too() {
        // Regression: CLAMP used to special-case an already-bounded Image
        // as a free pass-through, which meant a narrowed MIN/MAX silently
        // did nothing to it - only FloatImage input was ever actually
        // clamped. CLAMP must apply MIN/MAX uniformly regardless of input.
        let mut clamp = Clamp::new();
        clamp.set_parameter("MIN", Value::Number(0.5)).unwrap();
        let image = Arc::new(crate::graphics::U8Image {
            pixels: vec![0, 255, 0, 255], width: 1, height: 1, format: crate::graphics::ImageFormat::Rgba8,
        });

        let values = clamp
            .execute(&context(1, 1), &[(Input::Source, Value::Image(image))])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels[0], 128), // 0 crushed up to MIN (0.5 * 255, rounded)
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn set_parameter_rejects_a_min_that_would_cross_max() {
        let mut clamp = Clamp::new();
        let err = clamp.set_parameter("MIN", Value::Number(2.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn set_parameter_rejects_a_max_that_would_cross_min() {
        let mut clamp = Clamp::new();
        let err = clamp.set_parameter("MAX", Value::Number(-1.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn an_unwired_clamp_shows_the_missing_placeholder() {
        let clamp = Clamp::new();
        let values = clamp.execute(&context(2, 2), &[]).unwrap();
        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels.len(), (2 * 2 * 4) as usize),
            other => panic!("expected an image, got {:?}", other),
        }
    }
}
