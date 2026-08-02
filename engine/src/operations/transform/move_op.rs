// src/operations/transform/move_op.rs
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

/// Translates a node's own content by a fixed pixel offset, keeping the
/// frame's width/height unchanged - unlike RESIZE (which scales around the
/// frame's center), MOVE only repositions. Positive OFFSET_X/OFFSET_Y moves
/// content right/down. The region uncovered by the shift is fully
/// transparent, never wrapped or edge-clamped.
pub struct Move {
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Move {
    pub fn new() -> Self {
        Self { offset_x: 0.0, offset_y: 0.0 }
    }

    /// Nearest-neighbor translate of an RGBA buffer by (offset_x, offset_y)
    /// pixels. Pixels whose inverse-mapped source coordinate falls outside
    /// the original frame come out as transparent black, not wrapped or
    /// clamped to the edge - same edge behaviour as RESIZE's resize_pixels.
    ///
    /// Destination pixel (x,y) samples source (x - offset_x, y - offset_y) -
    /// same inverse-mapping convention resize_pixels uses (map destination
    /// to source, not source to destination).
    pub fn move_pixels<T: Copy + Default>(pixels: &[T], width: u32, height: u32, offset_x: f64, offset_y: f64) -> Vec<T> {
        let mut output = vec![T::default(); pixels.len()];

        for y in 0..height {
            for x in 0..width {
                let src_x = x as f64 - offset_x;
                let src_y = y as f64 - offset_y;

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

impl Default for Move {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Move {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "move",
            menu: "TRANSFORM",
            label: "MOVE",
            action: None,
            ui_action: None,
            create_node: Some("move"),
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
            display_name: "Move",
            category: OperationCategory::Color,
            // Unlike RESIZE, MOVE's output is always the same dimensions as
            // its input (translation, not a canvas resize) - so a MASK
            // input is valid here (no dimension-mismatch issue against
            // apply_mask), matching blur/invert/shuffle rather than
            // resize/clamp/rgb_to_hsv.
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
                name: "OFFSET_X",
                kind: ParameterKind::Number { step: 1.0, min: None, max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "OFFSET_Y",
                kind: ParameterKind::Number { step: 1.0, min: None, max: None },
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "OFFSET_X" => Some(Value::Number(self.offset_x)),
            "OFFSET_Y" => Some(Value::Number(self.offset_y)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("OFFSET_X", Value::Number(v)) => {
                self.offset_x = v;
                Ok(())
            }
            ("OFFSET_Y", Value::Number(v)) => {
                self.offset_y = v;
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

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        let moved = Self::move_pixels(&source.pixels, source.width, source.height, self.offset_x, self.offset_y);
        let moved = crate::graphics::apply_mask(&source.pixels, moved, mask.as_ref(), source.width, source.height)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: moved,
            width: source.width,
            height: source.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Move::new())
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
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<U8Image> {
        Arc::new(U8Image { pixels, width, height, format: ImageFormat::Rgba8 })
    }

    fn as_u8_pixels(value: &Value) -> Vec<u8> {
        match value {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels,
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn offset_zero_is_identity() {
        let pixels: Vec<u8> = (0..16).map(|n| (n * 16) as u8).collect(); // 2x2 RGBA
        let out = Move::move_pixels(&pixels, 2, 2, 0.0, 0.0);
        assert_eq!(out, pixels);
    }

    #[test]
    fn positive_offset_pads_the_uncovered_edge_with_transparency() {
        // 2x1 opaque image, shifted right by 1: x=0 becomes uncovered
        // (transparent), x=1 now shows what used to be at x=0 (opaque).
        let pixels = vec![10u8, 20, 30, 255, 40, 50, 60, 255];
        let out = Move::move_pixels(&pixels, 2, 1, 1.0, 0.0);

        assert_eq!(&out[0..4], &[0, 0, 0, 0], "uncovered edge must be transparent");
        assert_eq!(&out[4..8], &[10, 20, 30, 255], "shifted content must still be opaque");
    }

    #[test]
    fn offset_larger_than_frame_produces_a_fully_transparent_result() {
        let pixels: Vec<u8> = (0..(4 * 4)).flat_map(|_| [10, 20, 30, 255]).collect();
        let out = Move::move_pixels(&pixels, 4, 4, 100.0, 100.0);

        for chunk in out.chunks_exact(4) {
            assert_eq!(chunk, &[0, 0, 0, 0]);
        }
    }

    #[test]
    fn move_in_graph_is_valid() {
        let mut graph = Graph::new(4, 4);
        let move_id = graph.add_node(Box::new(Move::new()));
        graph.validate().expect("unwired move is valid");
        RenderExecutor::new()
            .execute(&graph, move_id, &context(4, 4))
            .expect("unwired move renders");
    }

    #[test]
    fn a_zero_alpha_mask_leaves_the_source_unmoved() {
        let mv = Move { offset_x: 1.0, offset_y: 0.0 };
        let input = image(vec![10, 20, 30, 255, 40, 50, 60, 255], 2, 1);
        let mask = image(vec![0, 0, 0, 0, 0, 0, 0, 0], 2, 1);

        let values = mv
            .execute(&context(2, 1), &[
                (Input::Source, Value::Image(input.clone())),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), input.pixels, "MASK=0 must reproduce the unmoved source exactly");
    }

    #[test]
    fn checkerboard_resize_move_produces_a_positioned_sized_rectangular_matte() {
        // The motivating use case: CHECKERBOARD (both colours equal, so it
        // reads as a solid rectangle) -> RESIZE (sized) -> MOVE (positioned)
        // - confirmed end to end through the real graph, not just each
        // operation's own unit tests in isolation.
        use crate::compositor::executors::{Execute, PreviewExecutor};
        use crate::graphics::Color;
        use crate::operations::generators::Checkerboard;
        use crate::operations::transform::Resize;

        let mut graph = Graph::new(8, 8);

        let mut checkerboard = Checkerboard::new();
        checkerboard.color_a = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        checkerboard.color_b = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let checkerboard_id = graph.add_node(Box::new(checkerboard));

        let mut resize = Resize::new();
        resize.set_parameter("SCALE_X", Value::Number(50.0)).unwrap(); // 4x4 opaque square, centered
        resize.set_parameter("SCALE_Y", Value::Number(50.0)).unwrap();
        let resize_id = graph.add_node(Box::new(resize));
        graph.connect(resize_id, Input::Source, checkerboard_id).unwrap();

        let mut mv = Move::new();
        mv.set_parameter("OFFSET_X", Value::Number(2.0)).unwrap();
        mv.set_parameter("OFFSET_Y", Value::Number(0.0)).unwrap();
        let move_id = graph.add_node(Box::new(mv));
        graph.connect(move_id, Input::Source, resize_id).unwrap();

        graph.validate().expect("the wired pipeline is valid");

        let values = PreviewExecutor::default()
            .execute(&graph, move_id, &context(8, 8))
            .unwrap();
        let pixels = as_u8_pixels(&values[0]);

        // Before the OFFSET_X=2 shift, RESIZE's 50% square sits at output
        // columns 2..6 (centered in an 8x8 frame). After shifting right by
        // 2, it must sit at columns 4..8, same rows.
        let pixel_at = |x: u32, y: u32| -> &[u8] {
            let index = ((y * 8 + x) * 4) as usize;
            &pixels[index..index + 4]
        };

        assert_eq!(pixel_at(2, 4), &[0, 0, 0, 0], "column 2 should now be uncovered (shifted away)");
        assert_eq!(pixel_at(5, 4), &[255, 255, 255, 255], "column 5 should now show the moved opaque square");
        assert_eq!(pixel_at(7, 4), &[255, 255, 255, 255], "column 7 is still within the square's new 4..8 extent");
        assert_eq!(pixel_at(0, 4), &[0, 0, 0, 0], "column 0 remains outside the square in every case");
    }

    #[test]
    fn a_full_alpha_mask_moves_exactly_as_unmasked() {
        let mv = Move { offset_x: 1.0, offset_y: 0.0 };
        let input = image(vec![10, 20, 30, 255, 40, 50, 60, 255], 2, 1);
        let mask = image(vec![0, 0, 0, 255, 0, 0, 0, 255], 2, 1);

        let unmasked = mv
            .execute(&context(2, 1), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();
        let masked = mv
            .execute(&context(2, 1), &[
                (Input::Source, Value::Image(input)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&unmasked[0]), as_u8_pixels(&masked[0]));
    }
}
