// src/operations/filter/blur.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    input::find_input,
    Operation,
    OperationDescriptor,
    metadata::{InputDescriptor, OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind, PIXEL_KINDS},
    Value,
};
use crate::graphics::FloatImage;

/// A simple separable box blur.
///
/// radius_px = 0 means “no blur” (identity).
/// radius_px > 0 applies a box kernel of width (2 * radius + 1).
pub struct Blur {
    pub radius_px: u32,
}

impl Blur {
    pub fn new() -> Self {
        Self { radius_px: 0 }
    }

    /// Apply a separable box blur to an RGBA buffer, in float - so
    /// blurring an already out-of-gamut FloatImage (e.g. an ADD result)
    /// averages its real values rather than whatever they'd have been
    /// clamped to first. A weighted average of in-range values can never
    /// itself go out of range, but it can (correctly) stay out of range
    /// if the input already was.
    ///
    /// This is a very basic implementation:
    /// - Horizontal pass, then vertical pass.
    /// - Clamps at edges (no special border modes).
    pub fn blur_pixels(&self, pixels: &[f32], width: u32, height: u32) -> Vec<f32> {
        if self.radius_px == 0 {
            return pixels.to_vec();
        }

        let w = width as usize;
        let h = height as usize;
        let r = self.radius_px as usize;

        let mut tmp = vec![0f32; pixels.len()];
        let mut out = vec![0f32; pixels.len()];

        // Horizontal pass
        for y in 0..h {
            let row_start = y * w * 4;
            for x in 0..w {
                let mut sum = [0f32; 4];
                let mut count = 0u32;

                let x_start = x.saturating_sub(r);
                let x_end = (x + r).min(w - 1);

                for xi in x_start..=x_end {
                    let idx = row_start + xi * 4;
                    sum[0] += pixels[idx];
                    sum[1] += pixels[idx + 1];
                    sum[2] += pixels[idx + 2];
                    sum[3] += pixels[idx + 3];
                    count += 1;
                }

                let inv = 1.0 / count as f32;
                let out_idx = row_start + x * 4;
                tmp[out_idx] = sum[0] * inv;
                tmp[out_idx + 1] = sum[1] * inv;
                tmp[out_idx + 2] = sum[2] * inv;
                tmp[out_idx + 3] = sum[3] * inv;
            }
        }

        // Vertical pass
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0f32; 4];
                let mut count = 0u32;

                let y_start = y.saturating_sub(r);
                let y_end = (y + r).min(h - 1);

                for yi in y_start..=y_end {
                    let idx = yi * w * 4 + x * 4;
                    sum[0] += tmp[idx];
                    sum[1] += tmp[idx + 1];
                    sum[2] += tmp[idx + 2];
                    sum[3] += tmp[idx + 3];
                    count += 1;
                }

                let inv = 1.0 / count as f32;
                let out_idx = y * w * 4 + x * 4;
                out[out_idx] = sum[0] * inv;
                out[out_idx + 1] = sum[1] * inv;
                out[out_idx + 2] = sum[2] * inv;
                out[out_idx + 3] = sum[3] * inv;
            }
        }

        out
    }
}

impl Default for Blur {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Blur {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "blur",
            menu: "TRANSFORM",
            label: "BLUR",
            action: None,
            ui_action: None,
            create_node: Some("blur"),
            submenu: Some("ASTRA"),
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
            display_name: "Blur",
            category: OperationCategory::Color,
            inputs: vec![
                InputDescriptor { kind: Input::Source, accepts: PIXEL_KINDS },
                InputDescriptor { kind: Input::Mask, accepts: PIXEL_KINDS },
            ],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![ParameterDescriptor {
            name: "RADIUS",
            // radius_px is stored as whole pixels (u32) - step by 1, not a
            // fraction that would never move the stored value.
            kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: Some(1000.0) },
            group: None,
        }]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "RADIUS" => Some(Value::Number(self.radius_px as f64)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("RADIUS", Value::Number(v)) => {
                if v < 0.0 || v > 1000.0 {
                    return Err(OperationError::InvalidParameterValue(
                        name.to_string(),
                        v.to_string(),
                    ));
                }
                self.radius_px = v.round() as u32;
                Ok(())
            }
            _ => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        let source = FloatImage::from_value(value, ctx)?;

        // Resolved once up front - MASK is independent of which concrete
        // Value variant SOURCE turns out to be.
        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        let blurred = self.blur_pixels(&source.pixels, source.width, source.height);
        let blurred = crate::graphics::apply_mask(&source.pixels, blurred, mask.as_ref(), source.width, source.height)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: blurred,
            width: source.width,
            height: source.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Blur::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::graph::Graph;
    use crate::compositor::executors::{Execute, RenderExecutor};
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
    fn zero_radius_is_identity() {
        let blur = Blur::new();
        let input = image(vec![10, 20, 30, 40, 50, 60, 70, 80], 2, 1);

        let values = blur
            .execute(&context(2, 1), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), input.pixels);
    }

    #[test]
    fn unconnected_blur_produces_the_missing_placeholder() {
        let blur = Blur::new();
        let values = blur.execute(&context(2, 1), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => {
                assert_eq!(out.width, 2);
                assert_eq!(out.height, 1);
                // Both pixels fall in the same 16px checker tile at this
                // tiny size, so they're the placeholder's magenta, not black.
                assert_eq!(out.pixels, vec![255, 0, 255, 255, 255, 0, 255, 255]);
            }
            other => panic!("expected image, got {:?}", other),
        }
    }

    #[test]
    fn blur_in_graph_is_valid() {
        let mut graph = Graph::new(4, 4);
        let blur_id = graph.add_node(Box::new(Blur::new()));
        graph.validate().expect("unwired blur is valid");
        RenderExecutor::new()
            .execute(&graph, blur_id, &context(4, 4))
            .expect("unwired blur renders");
    }

    #[test]
    fn setting_radius_by_the_stepper_step_actually_moves_it() {
        // The UI steps this parameter by 1.0 (RADIUS is stored as whole
        // pixels) - a step smaller than 1 would silently truncate away on
        // every set_parameter call, making the stepper look broken.
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(1.0)).unwrap();
        assert_eq!(blur.radius_px, 1);

        match blur.get_parameter("RADIUS") {
            Some(Value::Number(v)) => assert_eq!(v, 1.0),
            other => panic!("expected Value::Number(1.0), got {:?}", other),
        }
    }

    #[test]
    fn a_nonzero_radius_actually_blurs() {
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(1.0)).unwrap();

        let input = image(vec![255, 255, 255, 255, 0, 0, 0, 255], 2, 1);
        let values = blur
            .execute(&context(2, 1), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();

        assert_ne!(as_u8_pixels(&values[0]), input.pixels);
    }

    #[test]
    fn blurring_an_out_of_gamut_float_image_stays_out_of_gamut() {
        // A weighted average of already out-of-range values can correctly
        // still be out of range - blur must not silently clamp mid-average.
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(1.0)).unwrap();

        let input = Arc::new(FloatImage { pixels: vec![1.5, 1.5, 1.5, 1.0, 1.5, 1.5, 1.5, 1.0], width: 2, height: 1 });
        let values = blur
            .execute(&context(2, 1), &[(Input::Source, Value::FloatImage(input))])
            .unwrap();

        match &values[0] {
            Value::FloatImage(out) => assert!(out.is_out_of_gamut()),
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn a_zero_alpha_mask_suppresses_the_blur_entirely() {
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(5.0)).unwrap();

        let input = image(vec![255, 255, 255, 255, 0, 0, 0, 255], 2, 1);
        let mask = image(vec![0, 0, 0, 0, 0, 0, 0, 0], 2, 1);

        let values = blur
            .execute(&context(2, 1), &[
                (Input::Source, Value::Image(input.clone())),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), input.pixels, "MASK=0 must reproduce the unblurred source exactly");
    }

    #[test]
    fn a_full_alpha_mask_applies_the_blur_exactly_as_unmasked() {
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(1.0)).unwrap();

        let input = image(vec![255, 255, 255, 255, 0, 0, 0, 255], 2, 1);
        let mask = image(vec![0, 0, 0, 255, 0, 0, 0, 255], 2, 1);

        let unmasked = blur
            .execute(&context(2, 1), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();
        let masked = blur
            .execute(&context(2, 1), &[
                (Input::Source, Value::Image(input)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&unmasked[0]), as_u8_pixels(&masked[0]));
    }

    #[test]
    fn a_mismatched_mask_size_errors_instead_of_being_silently_ignored() {
        let blur = Blur::new();
        let input = image(vec![255, 255, 255, 255, 0, 0, 0, 255], 2, 1);
        let mask = image(vec![0, 0, 0, 255], 1, 1);

        let err = blur
            .execute(&context(2, 1), &[
                (Input::Source, Value::Image(input)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap_err();

        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }
}