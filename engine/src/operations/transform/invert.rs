// src/operations/transform/invert.rs
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
    metadata::{InputDescriptor, OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, PIXEL_KINDS},
    Value,
};
use crate::graphics::FloatImage;

/// Inverts every channel per pixel (1 - value), alpha included - matching
/// Multiply's convention of treating all 4 channels uniformly, which keeps
/// the blend-mode algebra exact (Screen(A,B) = Invert(Multiply(Invert(A),
/// Invert(B)))). Useful on its own, and as a building block for other blend
/// modes. Unclamped: inverting an out-of-gamut value (e.g. 1.5, from an
/// ADD result) correctly produces a negative one (-0.5), not 0.
pub struct Invert;

impl Invert {
    pub fn new() -> Self {
        Self
    }

    pub fn invert_pixels(pixels: &[f32]) -> Vec<f32> {
        pixels.iter().map(|channel| 1.0 - channel).collect()
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
            display_name: "Invert",
            category: OperationCategory::Color,
            inputs: vec![
                InputDescriptor { kind: Input::Source, accepts: PIXEL_KINDS },
                InputDescriptor { kind: Input::Mask, accepts: PIXEL_KINDS },
            ],
            outputs: vec![OutputKind::FloatImage],
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
            return Ok(vec![Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        let source = FloatImage::from_value(value, ctx)?;

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        // Phase 3 of BBOX_CONVENTIONS.md: with a MASK wired, apply_mask
        // below blends every pixel outside MASK's own real (nonzero-
        // weight) region straight back to `original` anyway - so restrict
        // the actual invert compute to MASK's own reported box alone.
        //
        // Deliberately NOT intersected with SOURCE's own reported box,
        // unlike BLUR: INVERT is not zero-preserving
        // (invert([0,0,0,0]) = [1,1,1,1], very much non-default) - a
        // "SOURCE has no real content here" box says nothing about where
        // INVERT's own output is non-default, since INVERT's output is
        // generically non-default *everywhere* regardless of SOURCE. This
        // is exactly why INVERT never overrode `output_bbox()` in Phase
        // 1/2: its own true natural box (ignoring MASK) already is
        // Rect::full, so intersecting with SOURCE's box here would be
        // invalid - it can silently skip real inversion in real,
        // mask-relevant pixels wherever SOURCE happens to report a
        // sub-frame box (caught by this operation's own regression test
        // before ever reaching review).
        let inverted = if mask.is_some() {
            let work_area = find_bbox(&ctx.input_bboxes, Input::Mask)
                .unwrap_or_else(|| Rect::full(source.width, source.height));

            let width = source.width;
            let pixels = &source.pixels;

            crate::graphics::compute_within_bbox(width, source.height, work_area, pixels, |x, y| {
                let idx = ((y * width + x) * 4) as usize;
                [1.0 - pixels[idx], 1.0 - pixels[idx + 1], 1.0 - pixels[idx + 2], 1.0 - pixels[idx + 3]]
            })
        } else {
            Self::invert_pixels(&source.pixels)
        };

        let inverted = crate::graphics::apply_mask(&source.pixels, inverted, mask.as_ref(), source.width, source.height)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: inverted,
            width: source.width,
            height: source.height,
        }))])
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
    fn inverting_black_is_white() {
        let black = image(vec![0, 0, 0, 128], 1, 1);
        let out = Invert::invert_pixels(&FloatImage::from_image(&black).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);
        assert_eq!(out.pixels, vec![255, 255, 255, 127]);
    }

    #[test]
    fn inverting_twice_is_identity() {
        let color = image(vec![10, 200, 50, 255], 1, 1);
        let start = FloatImage::from_image(&color).pixels;
        let once = Invert::invert_pixels(&start);
        let twice = Invert::invert_pixels(&once);
        let twice = FloatImage { pixels: twice, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);
        assert_eq!(twice.pixels, color.pixels);
    }

    #[test]
    fn inverting_an_out_of_gamut_value_goes_negative_not_to_zero() {
        // 1.5 inverted is -0.5, a real negative value - not clamped to 0
        // the way an 8-bit-only Invert would have to.
        let out = Invert::invert_pixels(&[1.5]);
        assert_eq!(out[0], -0.5);
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

        assert_eq!(as_u8_pixels(&values[0]), input.pixels);
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

        assert_eq!(as_u8_pixels(&values[0]), vec![245, 55, 205, 0]);
    }

    #[test]
    fn consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one() {
        let invert = Invert::new();

        let source = image((0..6).flat_map(|n| [n * 10, n * 10 + 1, n * 10 + 2, 255]).collect(), 6, 1);
        let mask = image(
            vec![
                0, 0, 0, 0,   0, 0, 0, 0,
                0, 0, 0, 255, 0, 0, 0, 255,
                0, 0, 0, 0,   0, 0, 0, 0,
            ],
            6, 1,
        );

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let ctx_with_real_box = Context {
            input_bboxes: vec![
                (Input::Source, Rect::full(6, 1)),
                (Input::Mask, Rect { x0: 2, y0: 0, x1: 4, y1: 1 }),
            ],
            ..context(6, 1)
        };
        let ctx_full_frame = context(6, 1);

        let restricted = invert.execute(&ctx_with_real_box, &inputs).unwrap();
        let unrestricted = invert.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box() {
        // Same lesson as BLUR's evaluator-caught bug: verify directly that
        // a sub-frame SOURCE box doesn't cause under-computation. Unlike
        // BLUR, INVERT's natural box is Rect::full, independent of SOURCE
        // entirely (see execute()'s own comment - INVERT isn't
        // zero-preserving, so SOURCE's box plays no role in work_area at
        // all) - this test confirms that holds, not just assumes it.
        let invert = Invert::new();

        let mut source_pixels = vec![0u8; 10 * 4];
        for x in 3..7 {
            source_pixels[x * 4..x * 4 + 4].copy_from_slice(&[100, 100, 100, 255]);
        }
        let source = image(source_pixels, 10, 1);
        let mask = image(vec![255; 10 * 4], 10, 1);

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let ctx_with_real_source_box = Context {
            input_bboxes: vec![
                (Input::Source, Rect { x0: 3, y0: 0, x1: 7, y1: 1 }),
                (Input::Mask, Rect::full(10, 1)),
            ],
            ..context(10, 1)
        };
        let ctx_full_frame = context(10, 1);

        let restricted = invert.execute(&ctx_with_real_source_box, &inputs).unwrap();
        let unrestricted = invert.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one() {
        use crate::graphics::mask::{reset_pixels_computed, take_pixels_computed};

        let invert = Invert::new();

        let source = image((0..16).flat_map(|n| [n, n, n, 255]).collect(), 4, 4);
        let mask = image(vec![255; 4 * 4 * 4], 4, 4);

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let small_box_ctx = Context {
            input_bboxes: vec![
                (Input::Source, Rect::full(4, 4)),
                (Input::Mask, Rect { x0: 1, y0: 1, x1: 2, y1: 2 }),
            ],
            ..context(4, 4)
        };
        reset_pixels_computed();
        invert.execute(&small_box_ctx, &inputs).unwrap();
        let small_box_pixels = take_pixels_computed().expect("INVERT with a wired MASK must record a pixel count");

        let full_frame_ctx = context(4, 4);
        reset_pixels_computed();
        invert.execute(&full_frame_ctx, &inputs).unwrap();
        let full_frame_pixels = take_pixels_computed().expect("INVERT with a wired MASK must record a pixel count");

        assert_eq!(small_box_pixels, 1);
        assert_eq!(full_frame_pixels, 16);
        assert!(small_box_pixels < full_frame_pixels);
    }

    #[test]
    fn checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor, RenderExecutor};
        use crate::graphics::Color;
        use crate::operations::generators::Checkerboard;
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::{Move, Resize};

        let mut graph = Graph::new(4, 4);

        let mut source = ImageSource::new();
        source.set_image(image((0..16).flat_map(|n| [n * 15, n * 15, n * 15, 255]).collect(), 4, 4));
        let source_id = graph.add_node(Box::new(source));

        let mut checkerboard = Checkerboard::new();
        checkerboard.color_a = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        checkerboard.color_b = checkerboard.color_a;
        let checkerboard_id = graph.add_node(Box::new(checkerboard));

        let mut resize = Resize::new();
        resize.set_parameter("SCALE_X", Value::Number(50.0)).unwrap();
        resize.set_parameter("SCALE_Y", Value::Number(50.0)).unwrap();
        let resize_id = graph.add_node(Box::new(resize));
        graph.connect(resize_id, Input::Source, checkerboard_id).unwrap();

        let move_id = graph.add_node(Box::new(Move::new()));
        graph.connect(move_id, Input::Source, resize_id).unwrap();

        let invert_id = graph.add_node(Box::new(Invert::new()));
        graph.connect(invert_id, Input::Source, source_id).unwrap();
        graph.connect(invert_id, Input::Mask, move_id).unwrap();

        graph.validate().expect("the wired pipeline is valid");

        let ctx = context(4, 4);

        let on_values = RenderExecutor::new().execute(&graph, invert_id, &ctx).unwrap();
        let on_pixels = as_u8_pixels(&on_values[0]);

        let source_value = PreviewExecutor::default().execute(&graph, source_id, &ctx).unwrap().into_iter().next().unwrap();
        let mask_value = PreviewExecutor::default().execute(&graph, move_id, &ctx).unwrap().into_iter().next().unwrap();

        let invert_off = Invert::new();
        let off_values = invert_off.execute(&ctx, &[
            (Input::Source, source_value),
            (Input::Mask, mask_value),
        ]).unwrap();
        let off_pixels = as_u8_pixels(&off_values[0]);

        assert_eq!(on_pixels, off_pixels, "bbox consumption on vs off must produce pixel-identical output");
    }
}
