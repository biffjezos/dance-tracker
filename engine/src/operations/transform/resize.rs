// src/operations/transform/resize.rs
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

/// Resampling algorithm for RESIZE. Only NEAREST_NEIGHBOR exists today - the
/// enum and the single-entry options list both exist already so adding
/// BILINEAR later is just a new match arm and a new string in the list, not
/// a parameter shape change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizeAlgorithm {
    NearestNeighbor,
}

pub const RESIZE_ALGORITHMS: &[&str] = &["NEAREST NEIGHBOR"];

impl Default for ResizeAlgorithm {
    fn default() -> Self {
        ResizeAlgorithm::NearestNeighbor
    }
}

impl ResizeAlgorithm {
    pub fn to_str(&self) -> &'static str {
        match self {
            ResizeAlgorithm::NearestNeighbor => "NEAREST NEIGHBOR",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "NEAREST NEIGHBOR" => Some(ResizeAlgorithm::NearestNeighbor),
            _ => None,
        }
    }
}

/// Scales a node's own content around its frame's center, keeping the
/// frame's width/height unchanged - a digital zoom, not a canvas resize.
/// Shrinking (scale < 100) leaves the uncovered edges fully transparent
/// (visible via the transparency checker); zooming in (scale > 100) always
/// samples from inside the original frame, so it crops rather than pads.
pub struct Resize {
    pub scale_x: f64,
    pub scale_y: f64,
    pub algorithm: ResizeAlgorithm,
}

impl Resize {
    pub fn new() -> Self {
        Self {
            scale_x: 100.0,
            scale_y: 100.0,
            algorithm: ResizeAlgorithm::default(),
        }
    }

    /// Nearest-neighbor resample of an RGBA buffer, scaled `scale_x`/`scale_y`
    /// percent around the frame's own center. Pixels whose inverse-mapped
    /// source coordinate falls outside the original frame come out as
    /// transparent black (all-zero/all-default), not clamped to the edge.
    ///
    /// Generic over the pixel element type (u8 or f32) - resampling is pure
    /// copying, no arithmetic on channel values, so there's nothing here
    /// that could ever produce an out-of-gamut result or need clamping
    /// either way; converting to float first would just be wasted work.
    pub fn resize_pixels<T: Copy + Default>(pixels: &[T], width: u32, height: u32, scale_x: f64, scale_y: f64) -> Vec<T> {
        let mut output = vec![T::default(); pixels.len()];

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let inv_x = 100.0 / scale_x;
        let inv_y = 100.0 / scale_y;

        for y in 0..height {
            for x in 0..width {
                let src_x = cx + (x as f64 + 0.5 - cx) * inv_x;
                let src_y = cy + (y as f64 + 0.5 - cy) * inv_y;

                if src_x < 0.0 || src_y < 0.0 || src_x >= width as f64 || src_y >= height as f64 {
                    continue;
                }

                let sx = src_x as u32;
                let sy = src_y as u32;

                let dest_index = ((y * width + x) * 4) as usize;
                let src_index = ((sy * width + sx) * 4) as usize;
                output[dest_index..dest_index + 4].copy_from_slice(&pixels[src_index..src_index + 4]);
            }
        }

        output
    }
}

impl Default for Resize {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Resize {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "resize",
            menu: "TRANSFORM",
            label: "RESIZE",
            action: None,
            ui_action: None,
            create_node: Some("resize"),
            submenu: Some("KINETIC"),
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
            display_name: "Resize",
            category: OperationCategory::Color,
            // Deliberately no Input::Mask: a MASK input needs to be the
            // same dimensions as both the "identity" pass-through and the
            // processed result it blends against (see graphics::apply_mask),
            // but Resize's own output is a *different* size than its input
            // at any scale != 100% - there's no single pixel-for-pixel
            // identity to blend a mask against here.
            inputs: vec![InputDescriptor { kind: Input::Source, accepts: PIXEL_KINDS }],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "SCALE_X",
                kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: Some(1000.0) },
                group: None,
            },
            ParameterDescriptor {
                name: "SCALE_Y",
                kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: Some(1000.0) },
                group: None,
            },
            ParameterDescriptor {
                name: "ALGORITHM",
                kind: ParameterKind::Enum(RESIZE_ALGORITHMS),
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "SCALE_X" => Some(Value::Number(self.scale_x)),
            "SCALE_Y" => Some(Value::Number(self.scale_y)),
            "ALGORITHM" => Some(Value::Text(self.algorithm.to_str().to_string())),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("SCALE_X", Value::Number(v)) => {
                if v < 1.0 || v > 1000.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.scale_x = v;
                Ok(())
            }
            ("SCALE_Y", Value::Number(v)) => {
                if v < 1.0 || v > 1000.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.scale_y = v;
                Ok(())
            }
            ("ALGORITHM", Value::Text(s)) => {
                self.algorithm = ResizeAlgorithm::from_str(&s)
                    .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        let source = FloatImage::from_value(value, ctx)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: Self::resize_pixels(&source.pixels, source.width, source.height, self.scale_x, self.scale_y),
            width: source.width,
            height: source.height,
        }))])
    }

    // Report-only (Phase 1 of BBOX_CONVENTIONS.md): remaps SOURCE's own
    // reported box through the exact inverse of resize_pixels's own
    // center-relative dest->src mapping - pure arithmetic, no pixel reads.
    // Rounds outward (floor the lower edge, ceil the upper edge) so the
    // reported box is never smaller than resize_pixels's true content
    // extent, per BBOX_CONVENTIONS.md's "larger is safe, smaller is not"
    // invariant. Result is clamped to the frame: at scale > 100 the
    // unclamped remap can exceed the frame, but resize_pixels itself never
    // writes outside it (see its own dest-bounded loop), so anything
    // outside the frame is never real content.
    fn output_bbox(&self, ctx: &Context, input_bboxes: &[(Input, Rect)], _output: &Value) -> Rect {
        let source_box = find_bbox(input_bboxes, Input::Source)
            .unwrap_or_else(|| Rect::full(ctx.meta.width, ctx.meta.height));

        if source_box.is_empty() {
            return Rect::empty();
        }

        let cx = ctx.meta.width as f64 / 2.0;
        let cy = ctx.meta.height as f64 / 2.0;

        let remap_x = |v: i32| -> f64 { cx + (v as f64 - cx) * (self.scale_x / 100.0) };
        let remap_y = |v: i32| -> f64 { cy + (v as f64 - cy) * (self.scale_y / 100.0) };

        let remapped = Rect {
            x0: remap_x(source_box.x0).floor() as i32,
            y0: remap_y(source_box.y0).floor() as i32,
            x1: remap_x(source_box.x1).ceil() as i32,
            y1: remap_y(source_box.y1).ceil() as i32,
        };

        remapped.intersect(&Rect::full(ctx.meta.width, ctx.meta.height))
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Resize::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::graph::Graph;
    use crate::compositor::executors::{Execute, RenderExecutor};

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn scale_100_is_identity() {
        let pixels: Vec<u8> = (0..16).map(|n| (n * 16) as u8).collect(); // 2x2 RGBA
        let out = Resize::resize_pixels(&pixels, 2, 2, 100.0, 100.0);
        assert_eq!(out, pixels);
    }

    #[test]
    fn shrinking_pads_edges_with_transparency() {
        // A solid opaque 4x4 image shrunk to 50% leaves a transparent
        // border around a smaller opaque center.
        let pixels: Vec<u8> = (0..4 * 4).flat_map(|_| [10, 20, 30, 255]).collect();
        let out = Resize::resize_pixels(&pixels, 4, 4, 50.0, 50.0);

        // Corner pixel maps outside the original frame once shrunk - transparent.
        assert_eq!(&out[0..4], &[0, 0, 0, 0]);

        // Center pixels still sample real (opaque) content.
        let center_index = ((2 * 4 + 2) * 4) as usize;
        assert_eq!(&out[center_index..center_index + 4], &[10, 20, 30, 255]);
    }

    #[test]
    fn zooming_in_never_pads_with_transparency() {
        let pixels = (0..(4 * 4))
            .flat_map(|_| [10u8, 20, 30, 255])
            .collect::<Vec<u8>>();
        let out = Resize::resize_pixels(&pixels, 4, 4, 200.0, 200.0);

        // Every pixel came from somewhere inside the original (opaque) frame.
        for chunk in out.chunks_exact(4) {
            assert_eq!(chunk[3], 255);
        }
    }

    #[test]
    fn set_parameter_rejects_an_out_of_range_scale() {
        let mut resize = Resize::new();
        let err = resize.set_parameter("SCALE_X", Value::Number(0.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn set_parameter_rejects_an_unknown_algorithm() {
        let mut resize = Resize::new();
        let err = resize.set_parameter("ALGORITHM", Value::Text("BILINEAR".into())).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn output_bbox_at_50_percent_on_a_full_frame_input_is_exactly_half_centered() {
        let mut resize = Resize::new();
        resize.set_parameter("SCALE_X", Value::Number(50.0)).unwrap();
        resize.set_parameter("SCALE_Y", Value::Number(50.0)).unwrap();

        let ctx = context(8, 8);
        let full = crate::compositor::bbox::Rect::full(8, 8);
        let bbox = resize.output_bbox(&ctx, &[(Input::Source, full)], &Value::Number(0.0));

        assert_eq!(bbox, crate::compositor::bbox::Rect { x0: 2, y0: 2, x1: 6, y1: 6 });
    }

    #[test]
    fn output_bbox_at_100_percent_on_a_full_frame_input_stays_full_frame() {
        let resize = Resize::new();
        let ctx = context(8, 8);
        let full = crate::compositor::bbox::Rect::full(8, 8);
        let bbox = resize.output_bbox(&ctx, &[(Input::Source, full)], &Value::Number(0.0));

        assert_eq!(bbox, full);
    }

    #[test]
    fn output_bbox_with_no_reported_source_box_defaults_to_full_frame() {
        // No Input::Source entry in input_bboxes at all - same as an
        // unwired SOURCE, or a SOURCE that never overrode output_bbox.
        let resize = Resize::new();
        let ctx = context(8, 8);
        let bbox = resize.output_bbox(&ctx, &[], &Value::Number(0.0));

        assert_eq!(bbox, crate::compositor::bbox::Rect::full(8, 8));
    }

    #[test]
    fn chaining_resize_into_an_unmodified_invert_is_still_pixel_identical() {
        // AC3: downstream operations that haven't opted into consuming
        // boxes yet (INVERT here) must still produce today's exact pixel
        // output - the new box being non-full-frame is metadata only.
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor};
        use crate::graphics::{ImageFormat, U8Image};
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::Invert;

        let mut graph = Graph::new(2, 1);

        let mut source = ImageSource::new();
        source.set_image(Arc::new(U8Image {
            pixels: vec![10, 20, 30, 255, 40, 50, 60, 255],
            width: 2,
            height: 1,
            format: ImageFormat::Rgba8,
        }));
        let source_id = graph.add_node(Box::new(source));

        let resize_id = graph.add_node(Box::new(Resize::new())); // scale=100, identity
        graph.connect(resize_id, Input::Source, source_id).unwrap();

        let invert_id = graph.add_node(Box::new(Invert::new()));
        graph.connect(invert_id, Input::Source, resize_id).unwrap();

        let values = PreviewExecutor::default()
            .execute(&graph, invert_id, &context(2, 1))
            .unwrap();

        let pixels = match &values[0] {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels.clone(),
            other => panic!("expected a float image, got {:?}", other),
        };

        assert_eq!(pixels, vec![245, 235, 225, 0, 215, 205, 195, 0], "INVERT's own unchanged execute() must still invert exactly as before");
    }

    #[test]
    fn resize_in_graph_is_valid() {
        let mut graph = Graph::new(4, 4);
        let resize_id = graph.add_node(Box::new(Resize::new()));
        graph.validate().expect("unwired resize is valid");
        RenderExecutor::new()
            .execute(&graph, resize_id, &context(4, 4))
            .expect("unwired resize renders");
    }
}
