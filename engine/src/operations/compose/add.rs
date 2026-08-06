// src/operations/compose/add.rs
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
    metadata::{InputDescriptor, OperationCategory, OperationMetadata, OutputKind, PIXEL_KINDS},
    Value,
};
use crate::graphics::FloatImage;

/// Add operation - adds RGBA channels from two inputs pixel by pixel,
/// unclamped: a sum above 1.0 is a legitimate out-of-gamut result (an
/// overexposed highlight, e.g.), same as any real compositor's Add/Plus
/// node - not an error to clip away here. CLAMP is the explicit,
/// deliberate step back down to a normal 0..1 Image. Same shape as
/// Multiply, other than that. Both inputs accept a bounded Image or an
/// already-unbounded FloatImage alike (via FloatImage::from_value), so
/// chaining ADD -> ADD works without an intervening CLAMP.
pub struct Add;

impl Add {
    pub fn new() -> Self {
        Self
    }

    /// Raw per-channel sum - NOT clamped. See this module's own doc
    /// comment for why.
    pub fn add_pixels(a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut output = vec![0f32; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            for channel in 0..4 {
                target[channel] = source_a[channel] + source_b[channel];
            }
        }

        output
    }

    /// The added value of a single pixel, computed directly from `a`/`b` -
    /// identical math to `add_pixels`'s own loop body for that index. Used
    /// by `execute()`'s bbox-restricted path (Phase 3 of
    /// BBOX_CONVENTIONS.md).
    fn add_single_pixel(a: &[f32], b: &[f32], x: u32, y: u32, width: u32) -> [f32; 4] {
        let idx = ((y * width + x) * 4) as usize;
        let mut out = [0f32; 4];
        for c in 0..4 {
            out[c] = a[idx + c] + b[idx + c];
        }
        out
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
            return Err(OperationError::InvalidInputType("Add requires first input".into()));
        };

        let Some(second) = find_input(inputs, Input::Background) else {
            return Err(OperationError::InvalidInputType("Add requires second input".into()));
        };

        let first_image = FloatImage::from_value(first, ctx)?;
        let second_image = FloatImage::from_value(second, ctx)?;

        if first_image.width != second_image.width || first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Add inputs must have matching dimensions".into()
            ));
        }

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        // Phase 3 of BBOX_CONVENTIONS.md: with a MASK wired, apply_mask
        // below blends every pixel outside the relevant region straight
        // back to Foreground (this operation's own "original", per
        // apply_mask's first argument below) anyway - so restrict the
        // actual add compute to the intersection of MASK's own reported
        // box and this operation's own natural box.
        //
        // Unlike BLUR/CHROMA KEY/SHUFFLE, that natural box is NOT simply
        // one input's own box: ADD is not zero-preserving on either input
        // alone - adding black to a real Background produces that real
        // Background unchanged (see adding_black_is_identity), so a pixel
        // where Foreground is default but Background is real is still
        // genuinely non-default output. The natural box is therefore the
        // UNION of Foreground's and Background's own reported boxes - the
        // region where EITHER input could contribute real content.
        let added = if mask.is_some() {
            let foreground_box = find_bbox(&ctx.input_bboxes, Input::Foreground)
                .unwrap_or_else(|| Rect::full(first_image.width, first_image.height));
            let background_box = find_bbox(&ctx.input_bboxes, Input::Background)
                .unwrap_or_else(|| Rect::full(first_image.width, first_image.height));
            let natural_box = foreground_box.union(&background_box);
            let mask_box = find_bbox(&ctx.input_bboxes, Input::Mask)
                .unwrap_or_else(|| Rect::full(first_image.width, first_image.height));
            let work_area = natural_box.intersect(&mask_box);

            let width = first_image.width;
            let a = &first_image.pixels;
            let b = &second_image.pixels;

            crate::graphics::compute_within_bbox(width, first_image.height, work_area, a, |x, y| {
                Self::add_single_pixel(a, b, x, y, width)
            })
        } else {
            Self::add_pixels(&first_image.pixels, &second_image.pixels)
        };

        let added = crate::graphics::apply_mask(&first_image.pixels, added, mask.as_ref(), first_image.width, first_image.height)?;

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

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<crate::graphics::U8Image> {
        Arc::new(crate::graphics::U8Image { pixels, width, height, format: crate::graphics::ImageFormat::Rgba8 })
    }

    fn float_pixels(pixels: Vec<u8>) -> Vec<f32> {
        FloatImage::from_image(&image(pixels, 1, 1)).pixels
    }

    #[test]
    fn adding_above_255_is_left_out_of_gamut_not_clamped() {
        let a = float_pixels(vec![200, 0, 100, 255]);
        let b = float_pixels(vec![100, 50, 200, 255]);

        let out = Add::add_pixels(&a, &b);

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
        let a = float_pixels(vec![100, 0, 50, 255]);
        let b = float_pixels(vec![50, 20, 30, 0]);

        let out = Add::add_pixels(&a, &b);
        let float_image = FloatImage { pixels: out, width: 1, height: 1 };
        let clamped = float_image.to_image_clamped(0.0, 1.0);

        assert_eq!(clamped.pixels, vec![150, 20, 80, 255]);
    }

    #[test]
    fn adding_black_is_identity() {
        let black = float_pixels(vec![0, 0, 0, 0]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Add::add_pixels(&black, &FloatImage::from_image(&color).pixels);
        let expected = FloatImage::from_image(&color).pixels;

        for (a, b) in out.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }

    #[test]
    fn chaining_add_into_add_accepts_the_out_of_gamut_float_image_input() {
        // Regression: ADD used to only accept a bounded Image/Frame/Video,
        // so wiring one ADD's output into another ADD errored out entirely.
        let inner = Add::add_pixels(&float_pixels(vec![200, 0, 100, 255]), &float_pixels(vec![200, 0, 100, 255]));
        let outer = Add::add_pixels(&inner, &inner);
        // 200+200+200+200 = 800, well out of gamut, and must not panic or
        // silently truncate.
        assert!((outer[0] - 800.0 / 255.0).abs() < 0.001);
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
                let expected = Add::add_pixels(&FloatImage::from_image(&fg).pixels, &FloatImage::from_image(&bg).pixels);
                for (a, b) in out.pixels.iter().zip(expected.iter()) {
                    assert!((a - b).abs() < 0.001);
                }
            }
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    fn as_u8_pixels(value: &Value) -> Vec<u8> {
        match value {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels,
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one() {
        let add = Add::new();

        let fg = image((0..6).flat_map(|n| [n * 10, n * 10 + 1, n * 10 + 2, 255]).collect(), 6, 1);
        let bg = image((0..6).flat_map(|n| [255 - n * 10, 100, 50, 255]).collect(), 6, 1);
        let mask = image(
            vec![
                0, 0, 0, 0,   0, 0, 0, 0,
                0, 0, 0, 255, 0, 0, 0, 255,
                0, 0, 0, 0,   0, 0, 0, 0,
            ],
            6, 1,
        );

        let inputs = [
            (Input::Foreground, Value::Image(fg)),
            (Input::Background, Value::Image(bg)),
            (Input::Mask, Value::Image(mask)),
        ];

        let ctx_with_real_box = Context {
            input_bboxes: vec![
                (Input::Foreground, Rect::full(6, 1)),
                (Input::Background, Rect::full(6, 1)),
                (Input::Mask, Rect { x0: 2, y0: 0, x1: 4, y1: 1 }),
            ],
            ..context(6, 1)
        };
        let ctx_full_frame = context(6, 1);

        let restricted = add.execute(&ctx_with_real_box, &inputs).unwrap();
        let unrestricted = add.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn consume_equivalence_requires_the_union_not_the_intersection_of_foreground_and_background_boxes() {
        // The load-bearing test for ADD specifically: unlike BLUR's
        // (evaluator-caught) mistake of using the wrong single-input box,
        // ADD's natural box must be the UNION of Foreground's and
        // Background's own boxes, not the intersection (and not just one
        // of them). Foreground is entirely default (black/transparent)
        // everywhere; Background carries the only real content, confined
        // to [3,7). If the code wrongly used only Foreground's box (empty
        // here) or the intersection (also empty, since Foreground's box
        // is empty), work_area would incorrectly become empty and the
        // real Background content would never get added in - the
        // restricted result would wrongly show Foreground (black)
        // everywhere instead of Foreground + Background's real colour.
        let add = Add::new();

        let fg = image(vec![0u8; 10 * 4], 10, 1);
        let mut bg_pixels = vec![0u8; 10 * 4];
        for x in 3..7 {
            bg_pixels[x * 4..x * 4 + 4].copy_from_slice(&[200, 150, 100, 255]);
        }
        let bg = image(bg_pixels, 10, 1);
        let mask = image(vec![255; 10 * 4], 10, 1);

        let inputs = [
            (Input::Foreground, Value::Image(fg)),
            (Input::Background, Value::Image(bg)),
            (Input::Mask, Value::Image(mask)),
        ];

        // Foreground reports an empty box (genuinely no real content
        // anywhere); Background reports its own real [3,7) box.
        let ctx_with_real_boxes = Context {
            input_bboxes: vec![
                (Input::Foreground, Rect::empty()),
                (Input::Background, Rect { x0: 3, y0: 0, x1: 7, y1: 1 }),
                (Input::Mask, Rect::full(10, 1)),
            ],
            ..context(10, 1)
        };
        let ctx_full_frame = context(10, 1);

        let restricted = add.execute(&ctx_with_real_boxes, &inputs).unwrap();
        let unrestricted = add.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));

        // Directly pin down that Background's real content actually got
        // added in, not silently skipped.
        let restricted_pixels = as_u8_pixels(&restricted[0]);
        assert_eq!(&restricted_pixels[4 * 4..4 * 4 + 4], &[200, 150, 100, 255], "Background's real content at x=4 must be added in, not left as Foreground's black");
    }

    #[test]
    fn a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one() {
        use crate::graphics::mask::{reset_pixels_computed, take_pixels_computed};

        let add = Add::new();

        let fg = image((0..16).flat_map(|n| [n, n, n, 255]).collect(), 4, 4);
        let bg = image((0..16).flat_map(|n| [255 - n, n, 128, 255]).collect(), 4, 4);
        let mask = image(vec![255; 4 * 4 * 4], 4, 4);

        let inputs = [
            (Input::Foreground, Value::Image(fg)),
            (Input::Background, Value::Image(bg)),
            (Input::Mask, Value::Image(mask)),
        ];

        let small_box_ctx = Context {
            input_bboxes: vec![
                (Input::Foreground, Rect::full(4, 4)),
                (Input::Background, Rect::full(4, 4)),
                (Input::Mask, Rect { x0: 1, y0: 1, x1: 2, y1: 2 }),
            ],
            ..context(4, 4)
        };
        reset_pixels_computed();
        add.execute(&small_box_ctx, &inputs).unwrap();
        let small_box_pixels = take_pixels_computed().expect("ADD with a wired MASK must record a pixel count");

        let full_frame_ctx = context(4, 4);
        reset_pixels_computed();
        add.execute(&full_frame_ctx, &inputs).unwrap();
        let full_frame_pixels = take_pixels_computed().expect("ADD with a wired MASK must record a pixel count");

        assert_eq!(small_box_pixels, 1);
        assert_eq!(full_frame_pixels, 16);
        assert!(small_box_pixels < full_frame_pixels);
    }

    #[test]
    fn checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor, RenderExecutor};
        use crate::operations::generators::Checkerboard;
        use crate::graphics::Color;
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::{Move, Resize};

        let mut graph = Graph::new(4, 4);

        let mut fg_source = ImageSource::new();
        fg_source.set_image(image((0..16).flat_map(|n| [n * 15, 0, 0, 255]).collect(), 4, 4));
        let fg_id = graph.add_node(Box::new(fg_source));

        let mut bg_source = ImageSource::new();
        bg_source.set_image(image((0..16).flat_map(|n| [0, n * 15, 0, 255]).collect(), 4, 4));
        let bg_id = graph.add_node(Box::new(bg_source));

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

        let add_id = graph.add_node(Box::new(Add::new()));
        graph.connect(add_id, Input::Foreground, fg_id).unwrap();
        graph.connect(add_id, Input::Background, bg_id).unwrap();
        graph.connect(add_id, Input::Mask, move_id).unwrap();

        graph.validate().expect("the wired pipeline is valid");

        let ctx = context(4, 4);

        let on_values = RenderExecutor::new().execute(&graph, add_id, &ctx).unwrap();
        let on_pixels = as_u8_pixels(&on_values[0]);

        let fg_value = PreviewExecutor::default().execute(&graph, fg_id, &ctx).unwrap().into_iter().next().unwrap();
        let bg_value = PreviewExecutor::default().execute(&graph, bg_id, &ctx).unwrap().into_iter().next().unwrap();
        let mask_value = PreviewExecutor::default().execute(&graph, move_id, &ctx).unwrap().into_iter().next().unwrap();

        let add_off = Add::new();
        let off_values = add_off.execute(&ctx, &[
            (Input::Foreground, fg_value),
            (Input::Background, bg_value),
            (Input::Mask, mask_value),
        ]).unwrap();
        let off_pixels = as_u8_pixels(&off_values[0]);

        assert_eq!(on_pixels, off_pixels, "bbox consumption on vs off must produce pixel-identical output");
    }
}
