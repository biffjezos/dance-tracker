// src/operations/filter/blur.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    bbox::Rect,
    Context,
    OperationError,
    Input,
    input::{find_bbox, find_input},
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

    // Report-only (Phase 2 of BBOX_CONVENTIONS.md): a box blur can spread
    // real content up to `radius_px` pixels beyond SOURCE's own reported
    // extent (every output pixel within `radius_px` of a real source
    // pixel can be pulled into that pixel's own average window), so the
    // reported box grows by exactly the kernel radius on every side, then
    // clamps to the frame - BLUR's own output is never larger than the
    // frame regardless of how far the grown box would otherwise extend.
    fn output_bbox(&self, ctx: &Context, input_bboxes: &[(Input, Rect)], _output: &Value) -> Rect {
        let source_box = find_bbox(input_bboxes, Input::Source)
            .unwrap_or_else(|| Rect::full(ctx.meta.width, ctx.meta.height));

        if source_box.is_empty() {
            return Rect::empty();
        }

        source_box
            .grow(self.radius_px as i32)
            .intersect(&Rect::full(ctx.meta.width, ctx.meta.height))
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
    fn output_bbox_grows_a_sub_frame_box_by_exactly_the_radius_clamped_to_the_frame() {
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(2.0)).unwrap();

        let ctx = context(10, 10);
        let sub_frame = crate::compositor::bbox::Rect { x0: 3, y0: 3, x1: 7, y1: 7 };
        let bbox = blur.output_bbox(&ctx, &[(Input::Source, sub_frame)], &Value::Number(0.0));

        // Grown by 2 on every side: [1,9) x [1,9) - well within the 10x10
        // frame, so no clamping kicks in yet.
        assert_eq!(bbox, crate::compositor::bbox::Rect { x0: 1, y0: 1, x1: 9, y1: 9 });
    }

    #[test]
    fn output_bbox_growth_past_the_frame_edge_is_clamped() {
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(5.0)).unwrap();

        let ctx = context(10, 10);
        let near_edge = crate::compositor::bbox::Rect { x0: 0, y0: 0, x1: 3, y1: 3 };
        let bbox = blur.output_bbox(&ctx, &[(Input::Source, near_edge)], &Value::Number(0.0));

        // Unclamped growth would be [-5,8) x [-5,8) - clamped to the
        // frame's own [0,10) x [0,10) on the low edge.
        assert_eq!(bbox, crate::compositor::bbox::Rect { x0: 0, y0: 0, x1: 8, y1: 8 });
    }

    #[test]
    fn output_bbox_of_an_already_full_frame_source_stays_full_frame_after_growth() {
        // Grow-then-clamp is a no-op at the frame edge - growing an
        // already-full-frame box can never make it any bigger than the
        // frame itself.
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(3.0)).unwrap();

        let ctx = context(10, 10);
        let full = crate::compositor::bbox::Rect::full(10, 10);
        let bbox = blur.output_bbox(&ctx, &[(Input::Source, full)], &Value::Number(0.0));

        assert_eq!(bbox, full);
    }

    #[test]
    fn output_bbox_with_no_reported_source_box_defaults_to_full_frame_then_grows_and_clamps() {
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(3.0)).unwrap();

        let ctx = context(10, 10);
        let bbox = blur.output_bbox(&ctx, &[], &Value::Number(0.0));

        assert_eq!(bbox, crate::compositor::bbox::Rect::full(10, 10));
    }

    #[test]
    fn chaining_blur_into_an_unmodified_invert_is_still_pixel_identical() {
        // Same AC3-style check Phase 1 used for RESIZE/MOVE: an unmodified
        // downstream operation (INVERT) must produce today's exact pixel
        // output regardless of BLUR's now-grown reported box.
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor};
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::Invert;

        let mut graph = Graph::new(2, 1);

        let mut source = ImageSource::new();
        source.set_image(image(vec![255, 255, 255, 255, 0, 0, 0, 255], 2, 1));
        let source_id = graph.add_node(Box::new(source));

        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(1.0)).unwrap();
        let blur_id = graph.add_node(Box::new(blur));
        graph.connect(blur_id, Input::Source, source_id).unwrap();

        let invert_id = graph.add_node(Box::new(Invert::new()));
        graph.connect(invert_id, Input::Source, blur_id).unwrap();

        let values = PreviewExecutor::default()
            .execute(&graph, invert_id, &context(2, 1))
            .unwrap();

        let pixels = as_u8_pixels(&values[0]);

        // BLUR radius=1 on a 2x1 image averages both pixels together for
        // every output pixel (each x's window covers both columns):
        // (255+0)/2=127(.5), alpha (255+255)/2=255. INVERT then inverts
        // every channel uniformly, unchanged from before this phase.
        assert_eq!(pixels, vec![128, 128, 128, 0, 128, 128, 128, 0]);
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