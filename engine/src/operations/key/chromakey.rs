// src/operations/key/chromakey.rs
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
use crate::graphics::{Color, FloatImage};

/// What an unconnected SOURCE shows: a flat, obviously-fake magenta - a
/// mask-producing node has nothing to key "removal" against, so the usual
/// missing()/transparency checker reads as more confusing than helpful
/// here. Not user-configurable; there's exactly one placeholder.
const PLACEHOLDER_COLOR: Color = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };

/// Chroma-key: cuts a pixel's alpha to 0 wherever its colour is within
/// THRESHOLD of KEY_COLOR, leaving everything else untouched. Distance is
/// plain Euclidean over normalized RGB, divided by sqrt(3) so it lands in
/// 0..1 regardless of which two colours are furthest apart (black/white).
pub struct ChromaKey {
    pub key_color: Color,
    pub threshold: f64,
}

impl ChromaKey {
    pub fn new() -> Self {
        Self {
            key_color: Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
            threshold: 0.3,
        }
    }

    /// Cut alpha to 0 for every pixel within `threshold` of `key_color`
    /// (both in normalized 0..1 RGB - unbounded input is measured as-is,
    /// no clamp), leaving RGB and any already-lower alpha untouched
    /// otherwise.
    pub fn key_pixels(pixels: &[f32], key_color: Color, threshold: f64) -> Vec<f32> {
        let key_r = key_color.r as f64;
        let key_g = key_color.g as f64;
        let key_b = key_color.b as f64;

        let mut output = vec![0f32; pixels.len()];

        for (source, target) in pixels.chunks_exact(4).zip(output.chunks_exact_mut(4)) {
            let r = source[0] as f64;
            let g = source[1] as f64;
            let b = source[2] as f64;

            let distance = ((r - key_r).powi(2) + (g - key_g).powi(2) + (b - key_b).powi(2)).sqrt()
                / 3f64.sqrt();

            target[0] = source[0];
            target[1] = source[1];
            target[2] = source[2];
            target[3] = if distance <= threshold { 0.0 } else { source[3] };
        }

        output
    }

    /// The keyed value of a single pixel, computed directly from `pixels`
    /// - identical math to `key_pixels`'s own loop body, just for one
    /// index. Used by `execute()`'s bbox-restricted path (Phase 3 of
    /// BBOX_CONVENTIONS.md).
    fn key_single_pixel(pixels: &[f32], key_color: Color, threshold: f64, x: u32, y: u32, width: u32) -> [f32; 4] {
        let key_r = key_color.r as f64;
        let key_g = key_color.g as f64;
        let key_b = key_color.b as f64;

        let idx = ((y * width + x) * 4) as usize;
        let r = pixels[idx] as f64;
        let g = pixels[idx + 1] as f64;
        let b = pixels[idx + 2] as f64;

        let distance = ((r - key_r).powi(2) + (g - key_g).powi(2) + (b - key_b).powi(2)).sqrt()
            / 3f64.sqrt();

        [
            pixels[idx],
            pixels[idx + 1],
            pixels[idx + 2],
            if distance <= threshold { 0.0 } else { pixels[idx + 3] },
        ]
    }
}

impl Default for ChromaKey {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ChromaKey {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "chromakey",
            menu: "KEY",
            label: "CHROMA KEY",
            action: None,
            ui_action: None,
            create_node: Some("chromakey"),
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
            display_name: "Chroma Key",
            category: OperationCategory::Mask,
            inputs: vec![
                InputDescriptor { kind: Input::Source, accepts: PIXEL_KINDS },
                InputDescriptor { kind: Input::Mask, accepts: PIXEL_KINDS },
            ],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "KEY_COLOR",
                kind: ParameterKind::Color,
                group: None,
            },
            ParameterDescriptor {
                name: "THRESHOLD",
                kind: ParameterKind::Number { step: 0.01, min: Some(0.0), max: Some(1.0) },
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "KEY_COLOR" => Some(Value::Color(self.key_color)),
            "THRESHOLD" => Some(Value::Number(self.threshold)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("KEY_COLOR", Value::Color(color)) => {
                self.key_color = color;
                Ok(())
            }
            ("THRESHOLD", Value::Number(v)) => {
                if !(0.0..=1.0).contains(&v) {
                    return Err(OperationError::InvalidParameterValue(
                        name.to_string(),
                        v.to_string(),
                    ));
                }
                self.threshold = v;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::solid(PLACEHOLDER_COLOR, ctx.meta.width, ctx.meta.height))]);
        };

        let source = FloatImage::from_value(value, ctx)?;

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        // Phase 3 of BBOX_CONVENTIONS.md: with a MASK wired, apply_mask
        // below blends every pixel outside the relevant region straight
        // back to `original` anyway - so restrict the actual keying
        // compute to the intersection of MASK's own reported box and
        // SOURCE's own reported box (no growth needed - key_pixels reads
        // only the pixel it writes, no neighbors). SOURCE's box is a
        // valid operand here: key_pixels is zero-preserving -
        // key_pixels([0,0,0,0]) is always [0,0,0,0] for any KEY_COLOR/
        // THRESHOLD, since RGB always passes through unchanged and alpha
        // is either explicitly zeroed or left at its already-zero value.
        // Only this operation's own wired MASK is in scope here - not
        // deriving a box from CHROMA KEY's own keyed-out alpha, which is
        // the excluded, content-derived Phase 4 (see PARKED_WORK.md).
        let keyed = if mask.is_some() {
            let natural_box = find_bbox(&ctx.input_bboxes, Input::Source)
                .unwrap_or_else(|| Rect::full(source.width, source.height));
            let mask_box = find_bbox(&ctx.input_bboxes, Input::Mask)
                .unwrap_or_else(|| Rect::full(source.width, source.height));
            let work_area = natural_box.intersect(&mask_box);

            let width = source.width;
            let pixels = &source.pixels;
            let key_color = self.key_color;
            let threshold = self.threshold;

            crate::graphics::compute_within_bbox(width, source.height, work_area, pixels, |x, y| {
                Self::key_single_pixel(pixels, key_color, threshold, x, y, width)
            })
        } else {
            Self::key_pixels(&source.pixels, self.key_color, self.threshold)
        };

        let keyed = crate::graphics::apply_mask(&source.pixels, keyed, mask.as_ref(), source.width, source.height)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: keyed,
            width: source.width,
            height: source.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(ChromaKey::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::{ImageFormat, U8Image};

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<U8Image> {
        Arc::new(U8Image { pixels, width, height, format: ImageFormat::Rgba8 })
    }

    #[test]
    fn mask_input_accepts_float_image_and_image() {
        let metadata = ChromaKey::new().metadata();
        let mask = metadata.inputs.iter().find(|d| d.kind == Input::Mask).unwrap();
        assert!(mask.accepts.contains(&OutputKind::FloatImage));
        assert!(mask.accepts.contains(&OutputKind::Image));
    }

    fn as_u8_pixels(value: &Value) -> Vec<u8> {
        match value {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels,
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn rewiring_source_through_the_graph_picks_up_the_new_value() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor};
        use crate::operations::sources::ImageSource;

        let ctx = Context {
            meta: crate::compositor::Meta { width: 4, height: 4, ..Default::default() },
            ..Default::default()
        };

        let mut graph = Graph::new(4, 4);

        let chroma_id = graph.add_node(Box::new(ChromaKey::new()));

        let mut src_a = ImageSource::new();
        src_a.set_image(image(vec![255, 0, 0, 255], 1, 1));
        let src_a_id = graph.add_node(Box::new(src_a));

        graph.connect(chroma_id, Input::Source, src_a_id).unwrap();

        let executor = PreviewExecutor::default();
        let values = executor.execute(&graph, chroma_id, &ctx).unwrap();
        assert_eq!(&as_u8_pixels(&values[0])[0..4], &[255, 0, 0, 255], "expected red from src_a");

        graph.disconnect(chroma_id, Input::Source).unwrap();
        let mut src_b = ImageSource::new();
        src_b.set_image(image(vec![0, 255, 0, 255], 1, 1));
        let src_b_id = graph.add_node(Box::new(src_b));
        graph.connect(chroma_id, Input::Source, src_b_id).unwrap();

        let values = executor.execute(&graph, chroma_id, &ctx).unwrap();
        assert_eq!(&as_u8_pixels(&values[0])[0..4], &[0, 255, 0, 0], "expected keyed-out green from src_b");
    }

    #[test]
    fn pure_green_is_keyed_out_at_default_settings() {
        let chromakey = ChromaKey::new();
        let green = FloatImage::from_image(&image(vec![0, 255, 0, 255], 1, 1));

        let out = ChromaKey::key_pixels(&green.pixels, chromakey.key_color, chromakey.threshold);

        assert_eq!(out, vec![0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn a_far_colour_is_left_alone() {
        let chromakey = ChromaKey::new();
        let red = FloatImage::from_image(&image(vec![255, 0, 0, 255], 1, 1));

        let out = ChromaKey::key_pixels(&red.pixels, chromakey.key_color, chromakey.threshold);

        assert_eq!(out, vec![1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn already_transparent_pixels_outside_the_key_stay_untouched() {
        let chromakey = ChromaKey::new();
        let translucent_red = FloatImage::from_image(&image(vec![255, 0, 0, 100], 1, 1));

        let out = ChromaKey::key_pixels(&translucent_red.pixels, chromakey.key_color, chromakey.threshold);

        assert_eq!(out[3], translucent_red.pixels[3]);
    }

    #[test]
    fn threshold_zero_only_keys_the_exact_colour() {
        let key_color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
        let exact = FloatImage::from_image(&image(vec![0, 255, 0, 255], 1, 1));
        let close = FloatImage::from_image(&image(vec![10, 245, 10, 255], 1, 1));

        assert_eq!(ChromaKey::key_pixels(&exact.pixels, key_color, 0.0), vec![0.0, 1.0, 0.0, 0.0]);
        assert_eq!(ChromaKey::key_pixels(&close.pixels, key_color, 0.0), close.pixels);
    }

    #[test]
    fn set_parameter_updates_key_color_and_threshold() {
        let mut chromakey = ChromaKey::new();
        chromakey.set_parameter("KEY_COLOR", Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 })).unwrap();
        chromakey.set_parameter("THRESHOLD", Value::Number(0.5)).unwrap();

        assert_eq!(chromakey.key_color, Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        assert_eq!(chromakey.threshold, 0.5);
    }

    #[test]
    fn set_parameter_rejects_out_of_range_threshold() {
        let mut chromakey = ChromaKey::new();
        let err = chromakey.set_parameter("THRESHOLD", Value::Number(1.5)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn an_unconnected_chromakey_is_solid_by_default_not_the_missing_checker() {
        let chromakey = ChromaKey::new();
        let values = chromakey.execute(&context(2, 1), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => {
                // Solid pink, not the missing()-style checker (which would
                // alternate magenta/black between pixels).
                assert_eq!(out.pixels, vec![255, 0, 255, 255, 255, 0, 255, 255]);
            }
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn a_zero_alpha_mask_suppresses_keying_entirely() {
        let chromakey = ChromaKey::new();
        let green = image(vec![0, 255, 0, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 0], 1, 1);

        let values = chromakey
            .execute(&context(1, 1), &[
                (Input::Source, Value::Image(green.clone())),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), green.pixels, "MASK=0 must leave the source unkeyed");
    }

    #[test]
    fn a_full_alpha_mask_keys_exactly_as_unmasked() {
        let chromakey = ChromaKey::new();
        let green = image(vec![0, 255, 0, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 255], 1, 1);

        let values = chromakey
            .execute(&context(1, 1), &[
                (Input::Source, Value::Image(green)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), vec![0, 255, 0, 0]);
    }

    #[test]
    fn consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one() {
        let chromakey = ChromaKey::new();

        // A mix of pure green (keys out) and other colours (don't), so
        // the restricted vs. unrestricted comparison is meaningful.
        let source = image(
            vec![
                255, 0, 0, 255,  0, 255, 0, 255,
                0, 255, 0, 255,  10, 20, 30, 255,
                255, 255, 255, 255, 0, 0, 0, 255,
            ],
            6, 1,
        );
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

        let restricted = chromakey.execute(&ctx_with_real_box, &inputs).unwrap();
        let unrestricted = chromakey.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box() {
        // Verifies key_pixels's zero-preservation directly: RGB always
        // passes through unchanged and alpha is either explicitly zeroed
        // or left at its already-zero value, so key_pixels([0,0,0,0]) is
        // always [0,0,0,0] regardless of KEY_COLOR/THRESHOLD - SOURCE's
        // own box is therefore a valid intersection operand here.
        let chromakey = ChromaKey::new(); // key_color = pure green, threshold 0.3

        let mut source_pixels = vec![0u8; 10 * 4];
        // Real content inside [3,7): pure green (keys out) at x=4, red
        // (stays) at x=5, so the restricted region itself has a mix.
        source_pixels[3 * 4..3 * 4 + 4].copy_from_slice(&[0, 255, 0, 255]);
        source_pixels[4 * 4..4 * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
        source_pixels[5 * 4..5 * 4 + 4].copy_from_slice(&[10, 20, 30, 255]);
        source_pixels[6 * 4..6 * 4 + 4].copy_from_slice(&[0, 255, 0, 255]);
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

        let restricted = chromakey.execute(&ctx_with_real_source_box, &inputs).unwrap();
        let unrestricted = chromakey.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one() {
        use crate::graphics::mask::{reset_pixels_computed, take_pixels_computed};

        let chromakey = ChromaKey::new();

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
        chromakey.execute(&small_box_ctx, &inputs).unwrap();
        let small_box_pixels = take_pixels_computed().expect("CHROMA KEY with a wired MASK must record a pixel count");

        let full_frame_ctx = context(4, 4);
        reset_pixels_computed();
        chromakey.execute(&full_frame_ctx, &inputs).unwrap();
        let full_frame_pixels = take_pixels_computed().expect("CHROMA KEY with a wired MASK must record a pixel count");

        assert_eq!(small_box_pixels, 1);
        assert_eq!(full_frame_pixels, 16);
        assert!(small_box_pixels < full_frame_pixels);
    }

    #[test]
    fn checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor, RenderExecutor};
        use crate::operations::generators::Checkerboard;
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::{Move, Resize};

        let mut graph = Graph::new(4, 4);

        let mut source = ImageSource::new();
        source.set_image(image((0..16).flat_map(|n| [n * 15, 0, 0, 255]).collect(), 4, 4));
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

        let chromakey_id = graph.add_node(Box::new(ChromaKey::new()));
        graph.connect(chromakey_id, Input::Source, source_id).unwrap();
        graph.connect(chromakey_id, Input::Mask, move_id).unwrap();

        graph.validate().expect("the wired pipeline is valid");

        let ctx = context(4, 4);

        let on_values = RenderExecutor::new().execute(&graph, chromakey_id, &ctx).unwrap();
        let on_pixels = as_u8_pixels(&on_values[0]);

        let source_value = PreviewExecutor::default().execute(&graph, source_id, &ctx).unwrap().into_iter().next().unwrap();
        let mask_value = PreviewExecutor::default().execute(&graph, move_id, &ctx).unwrap().into_iter().next().unwrap();

        let chromakey_off = ChromaKey::new();
        let off_values = chromakey_off.execute(&ctx, &[
            (Input::Source, source_value),
            (Input::Mask, mask_value),
        ]).unwrap();
        let off_pixels = as_u8_pixels(&off_values[0]);

        assert_eq!(on_pixels, off_pixels, "bbox consumption on vs off must produce pixel-identical output");
    }
}
