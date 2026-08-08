// src/operations/transform/move_op.rs
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
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
    value::value_ptr_eq,
    Value,
};
use crate::graphics::FloatImage;
use crate::gpu::GpuState;

// GPU compute shader - SPECwebgpuoperations.md's Phase 2 (resampling).
// Same "unmigrated to bbox-consumption" situation as MULTIPLY (Phase
// 1.3): MOVE's execute() masked path is still an unrestricted full-frame
// move_pixels + apply_mask, not a work_area-restricted one - see
// find_bbox/compute_within_bbox appearing only in output_bbox() below,
// never in execute() itself. GPU dispatch still applies only when
// `mask.is_none()`, same blanket-rule split as every other operation -
// the masked path is left entirely untouched (still CPU, still
// unrestricted). Coordinate math is a direct port of move_pixels's own
// simple offset inverse-mapping (destination -> source), same structure
// as RESIZE's shader but without the center-relative scale term.
const MOVE_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> input: array<f32>;
    @group(0) @binding(1) var<storage, read_write> output: array<f32>;
    @group(0) @binding(2) var<uniform> params: vec4<u32>;

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params.x;
        let height = params.y;
        let offset_x = bitcast<f32>(params.z);
        let offset_y = bitcast<f32>(params.w);

        if (id.x >= width || id.y >= height) {
            return;
        }

        let src_x = f32(id.x) - offset_x;
        let src_y = f32(id.y) - offset_y;

        let out_idx = (id.y * width + id.x) * 4u;

        if (src_x < 0.0 || src_y < 0.0 || src_x >= f32(width) || src_y >= f32(height)) {
            output[out_idx] = 0.0;
            output[out_idx + 1u] = 0.0;
            output[out_idx + 2u] = 0.0;
            output[out_idx + 3u] = 0.0;
            return;
        }

        let sx = u32(src_x);
        let sy = u32(src_y);
        let src_idx = (sy * width + sx) * 4u;

        output[out_idx] = input[src_idx];
        output[out_idx + 1u] = input[src_idx + 1u];
        output[out_idx + 2u] = input[src_idx + 2u];
        output[out_idx + 3u] = input[src_idx + 3u];
    }
"#;

struct MoveGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_move_pipeline(gpu: &GpuState) -> MoveGpuPipeline {
    let shader = gpu.create_shader(MOVE_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("move bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline = gpu.create_compute_pipeline("move pipeline", &shader, "main", &[&bind_group_layout]);

    MoveGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct MoveFingerprint {
    source: Value,
    offset_x_bits: u64,
    offset_y_bits: u64,
}

impl MoveFingerprint {
    fn matches(&self, other: &MoveFingerprint) -> bool {
        self.offset_x_bits == other.offset_x_bits && self.offset_y_bits == other.offset_y_bits && value_ptr_eq(&self.source, &other.source)
    }
}

struct CompletedMoveJob {
    fingerprint: MoveFingerprint,
    pixels: Vec<f32>,
    width: u32,
    height: u32,
}

/// Translates a node's own content by a fixed pixel offset, keeping the
/// frame's width/height unchanged - unlike RESIZE (which scales around the
/// frame's center), MOVE only repositions. Positive OFFSET_X/OFFSET_Y moves
/// content right/down. The region uncovered by the shift is fully
/// transparent, never wrapped or edge-clamped.
pub struct Move {
    pub offset_x: f64,
    pub offset_y: f64,
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale (Rc so the wasm32 spawn_local task can share the
    // cell without requiring `self` to be 'static).
    gpu_pipeline: RefCell<Option<MoveGpuPipeline>>,
    pending: Rc<RefCell<Option<MoveFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedMoveJob>>>,
}

impl Move {
    pub fn new() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Multiply::dispatch_gpu` in structure (single-buffer input,
    /// same as RESIZE's own dispatch_gpu). `offset_x`/`offset_y` are
    /// uploaded as bitcast-f32 uniform values, matching move_pixels's own
    /// f64 formula exactly before the f32 cast.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: MoveFingerprint, source: FloatImage, offset_x: f64, offset_y: f64) {
        let width = source.width;
        let height = source.height;
        let len = source.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_move_pipeline(&gpu));
            }
        }

        let input_buffer = gpu.upload("move input", &source.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "move output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );

        let offset_x_f32 = offset_x as f32;
        let offset_y_f32 = offset_y as f32;
        let params: [u32; 4] = [width, height, offset_x_f32.to_bits(), offset_y_f32.to_bits()];
        let params_buffer = gpu.upload("move params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "move readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("move bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("move dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pixels = gpu.read_buffer_blocking(&readback_buffer, len);
            *self.last_gpu_result.borrow_mut() = Some(CompletedMoveJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pixels = gpu.read_buffer_async(&readback_buffer, len).await;
                *last_gpu_result.borrow_mut() = Some(CompletedMoveJob {
                    fingerprint: fingerprint.clone(),
                    pixels,
                    width,
                    height,
                });
                let mut pending_slot = pending.borrow_mut();
                if pending_slot.as_ref().is_some_and(|p| p.matches(&fingerprint)) {
                    *pending_slot = None;
                }
            });
        }
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

    // See blur.rs's identical override for the full rationale - a pending
    // GPU dispatch must force re-execution or a completed result can get
    // stranded behind RenderExecutor's cross-tick cache.
    fn is_live(&self) -> bool {
        self.pending.borrow().is_some()
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        // MOVE isn't migrated to bbox-consumption (see this module's own
        // GPU shader doc comment) - the masked path stays exactly as it
        // was, full-frame CPU compute + apply_mask, completely untouched
        // by GPU dispatch.
        if let Some(mask) = &mask {
            let source = FloatImage::from_value(value, ctx)?;
            let moved = Self::move_pixels(&source.pixels, source.width, source.height, self.offset_x, self.offset_y);
            let moved = crate::graphics::apply_mask(&source.pixels, moved, Some(mask), source.width, source.height)?;

            return Ok(vec![Value::FloatImage(Arc::new(FloatImage {
                pixels: moved,
                width: source.width,
                height: source.height,
            }))]);
        }

        // Unmasked path: try GPU first when available.
        if let Some(gpu) = ctx.gpu.clone() {
            let fingerprint = MoveFingerprint {
                source: value.clone(),
                offset_x_bits: self.offset_x.to_bits(),
                offset_y_bits: self.offset_y.to_bits(),
            };

            let cached = self.last_gpu_result.borrow().as_ref()
                .filter(|completed| completed.fingerprint.matches(&fingerprint))
                .map(|completed| FloatImage { pixels: completed.pixels.clone(), width: completed.width, height: completed.height });

            if let Some(result) = cached {
                return Ok(vec![Value::FloatImage(Arc::new(result))]);
            }

            let already_pending = self.pending.borrow().as_ref().is_some_and(|p| p.matches(&fingerprint));
            if !already_pending {
                let source = FloatImage::from_value(value, ctx)?;
                self.dispatch_gpu(gpu, fingerprint, source, self.offset_x, self.offset_y);
            }
        }

        let source = FloatImage::from_value(value, ctx)?;
        let moved = Self::move_pixels(&source.pixels, source.width, source.height, self.offset_x, self.offset_y);

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: moved,
            width: source.width,
            height: source.height,
        }))])
    }

    // Report-only (Phase 1 of BBOX_CONVENTIONS.md): translates SOURCE's own
    // reported box by exactly (OFFSET_X, OFFSET_Y) - pure arithmetic, no
    // pixel reads - then clamps to the frame. An offset that moves the
    // whole box off-canvas correctly collapses to Rect::empty() via the
    // intersect below, never a box with negative extent.
    //
    // Rounds the *translated continuous bound* outward (floor the lower
    // edge, ceil the upper edge), not the offset itself - rounding the
    // offset first (e.g. offset_x=0.4 -> 0) can disagree with move_pixels's
    // own exact, unrounded, truncating-sample math and undershoot the true
    // content extent (a real regression, caught by evaluator review: see
    // the fractional-offset regression test below).
    fn output_bbox(&self, ctx: &Context, input_bboxes: &[(Input, Rect)], _output: &Value) -> Rect {
        let source_box = find_bbox(input_bboxes, Input::Source)
            .unwrap_or_else(|| Rect::full(ctx.meta.width, ctx.meta.height));

        if source_box.is_empty() {
            return Rect::empty();
        }

        let translated = Rect {
            x0: (source_box.x0 as f64 + self.offset_x).floor() as i32,
            y0: (source_box.y0 as f64 + self.offset_y).floor() as i32,
            x1: (source_box.x1 as f64 + self.offset_x).ceil() as i32,
            y1: (source_box.y1 as f64 + self.offset_y).ceil() as i32,
        };

        translated.intersect(&Rect::full(ctx.meta.width, ctx.meta.height))
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
    fn output_bbox_translates_a_full_frame_input_by_exactly_the_offset_clamped_to_the_frame() {
        let mv = Move { offset_x: 2.0, offset_y: 1.0, ..Move::new() };
        let ctx = context(8, 8);
        let full = crate::compositor::bbox::Rect::full(8, 8);
        let bbox = mv.output_bbox(&ctx, &[(Input::Source, full)], &Value::Number(0.0));

        // Unclamped translation would be [2,10) x [1,9) - clamped to the
        // frame's own [0,8) x [0,8).
        assert_eq!(bbox, crate::compositor::bbox::Rect { x0: 2, y0: 1, x1: 8, y1: 8 });
    }

    #[test]
    fn output_bbox_with_an_offset_larger_than_the_frame_is_empty_not_negative_extent() {
        let mv = Move { offset_x: 100.0, offset_y: 0.0, ..Move::new() };
        let ctx = context(8, 8);
        let full = crate::compositor::bbox::Rect::full(8, 8);
        let bbox = mv.output_bbox(&ctx, &[(Input::Source, full)], &Value::Number(0.0));

        assert!(bbox.is_empty(), "an offset moving the whole box off-canvas must report Rect::empty(), not a negative-extent rect");
    }

    #[test]
    fn output_bbox_with_a_fractional_offset_never_undershoots_move_pixels_real_content() {
        // Regression (evaluator-caught): rounding OFFSET_X/Y before
        // translating the box could disagree with move_pixels's own exact,
        // truncating-sample math and report a box smaller than the true
        // content extent. width=4, SOURCE box [0,1) (only source pixel 0 is
        // real content), offset_x=0.4: move_pixels puts real content at
        // dest_x=1 (src_x = 1 - 0.4 = 0.6, truncates to source pixel 0,
        // which is inside the box) - the reported box must cover dest_x=1.
        let mv = Move { offset_x: 0.4, offset_y: 0.0, ..Move::new() };
        let ctx = context(4, 1);
        let source_box = crate::compositor::bbox::Rect { x0: 0, y0: 0, x1: 1, y1: 1 };

        let bbox = mv.output_bbox(&ctx, &[(Input::Source, source_box)], &Value::Number(0.0));

        assert!(
            bbox.x0 <= 1 && bbox.x1 > 1,
            "reported box {:?} must cover dest_x=1, where move_pixels actually writes real content for offset_x=0.4",
            bbox
        );

        // Cross-check directly against move_pixels itself: the only
        // opaque source pixel is x=0; confirm which dest pixels actually
        // receive real content, and assert the box covers all of them.
        let source_pixels = vec![10u8, 20, 30, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let moved = Move::move_pixels(&source_pixels, 4, 1, 0.4, 0.0);
        for x in 0..4u32 {
            let px = &moved[(x * 4) as usize..(x * 4 + 4) as usize];
            if px != [0, 0, 0, 0] {
                assert!(
                    bbox.x0 <= x as i32 && bbox.x1 > x as i32,
                    "move_pixels wrote real content at x={} but reported box {:?} doesn't cover it",
                    x, bbox
                );
            }
        }
    }

    #[test]
    fn output_bbox_with_no_reported_source_box_defaults_to_full_frame_then_translates() {
        let mv = Move { offset_x: 1.0, offset_y: 0.0, ..Move::new() };
        let ctx = context(8, 8);
        let bbox = mv.output_bbox(&ctx, &[], &Value::Number(0.0));

        assert_eq!(bbox, crate::compositor::bbox::Rect { x0: 1, y0: 0, x1: 8, y1: 8 });
    }

    #[test]
    fn chaining_move_into_an_unmodified_invert_is_still_pixel_identical() {
        // AC3: an unmodified downstream operation (INVERT) must still
        // produce today's exact pixel output regardless of MOVE's now
        // non-full-frame reported box.
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor};
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::Invert;

        let mut graph = Graph::new(2, 1);

        let mut source = ImageSource::new();
        source.set_image(image(vec![10, 20, 30, 255, 40, 50, 60, 255], 2, 1));
        let source_id = graph.add_node(Box::new(source));

        let mut mv = Move::new();
        mv.set_parameter("OFFSET_X", Value::Number(1.0)).unwrap();
        let move_id = graph.add_node(Box::new(mv));
        graph.connect(move_id, Input::Source, source_id).unwrap();

        let invert_id = graph.add_node(Box::new(Invert::new()));
        graph.connect(invert_id, Input::Source, move_id).unwrap();

        let values = PreviewExecutor::default()
            .execute(&graph, invert_id, &context(2, 1))
            .unwrap();

        let pixels = as_u8_pixels(&values[0]);

        // MOVE shifts right by 1: x=0 becomes transparent black (moved's
        // own uncovered edge), x=1 shows the original x=0 pixel (10,20,30,255).
        // INVERT then inverts every channel uniformly, unchanged from before.
        assert_eq!(pixels, vec![255, 255, 255, 255, 245, 235, 225, 0]);
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
        let mv = Move { offset_x: 1.0, offset_y: 0.0, ..Move::new() };
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
        let mv = Move { offset_x: 1.0, offset_y: 0.0, ..Move::new() };
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

    // --- WebGPU Phase 2 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let mv = Move::new();
        assert!(!mv.is_live(), "no dispatch in flight yet - must not force re-execution");

        *mv.pending.borrow_mut() = Some(MoveFingerprint {
            source: Value::Number(0.0),
            offset_x_bits: 1.0_f64.to_bits(),
            offset_y_bits: 0.0_f64.to_bits(),
        });
        assert!(mv.is_live(), "a pending GPU dispatch must force re-execution so a just-completed result gets picked up");

        *mv.pending.borrow_mut() = None;
        assert!(!mv.is_live(), "once nothing is pending, normal cross-tick caching should resume");
    }

    #[test]
    fn gpu_move_matches_cpu_within_tolerance_once_warmed_up() {
        let Ok(gpu) = pollster::block_on(crate::gpu::GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };
        let gpu = Arc::new(gpu);

        let width = 6;
        let height = 5;
        let pixels: Vec<u8> = (0..(width * height))
            .flat_map(|n| {
                let v = ((n * 37) % 256) as u8;
                [v, v.wrapping_add(50), v.wrapping_add(100), 255]
            })
            .collect();
        let input = image(pixels, width, height);

        let mut cpu_move = Move::new();
        cpu_move.set_parameter("OFFSET_X", Value::Number(2.0)).unwrap();
        cpu_move.set_parameter("OFFSET_Y", Value::Number(-1.0)).unwrap();
        let cpu_values = cpu_move
            .execute(&context(width, height), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();
        let Value::FloatImage(cpu_result) = &cpu_values[0] else { panic!("expected a float image") };

        let mut gpu_move = Move::new();
        gpu_move.set_parameter("OFFSET_X", Value::Number(2.0)).unwrap();
        gpu_move.set_parameter("OFFSET_Y", Value::Number(-1.0)).unwrap();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };

        // First call: no cached GPU result yet - kicks off a dispatch
        // (native resolves it synchronously inside dispatch_gpu) and
        // falls back to CPU for this tick regardless.
        let _ = gpu_move.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        // Second call, same fingerprint (same wired Arc, same offsets):
        // the now-completed GPU result is cached and used directly.
        let gpu_values = gpu_move.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        let Value::FloatImage(gpu_result) = &gpu_values[0] else { panic!("expected a float image") };

        assert_eq!(cpu_result.pixels.len(), gpu_result.pixels.len());
        for (index, (cpu_px, gpu_px)) in cpu_result.pixels.iter().zip(gpu_result.pixels.iter()).enumerate() {
            assert!(
                (cpu_px - gpu_px).abs() < 1e-4,
                "channel {}: cpu={}, gpu={}",
                index,
                cpu_px,
                gpu_px
            );
        }
    }
}
