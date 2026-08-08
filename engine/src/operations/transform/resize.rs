// src/operations/transform/resize.rs
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
// No MASK input exists for this operation (see metadata()'s own
// comment), so GPU dispatch applies unconditionally when available -
// same shape as RGB_TO_HSV/CHECKERBOARD in that regard. Unlike every
// prior phase's pointwise shaders, this one computes a *transformed*
// source address per invocation (destination -> source, inverse-mapped)
// rather than reading its own (x, y) - the WGSL body is a direct port of
// resize_pixels's own coordinate math, done in f32 (GPU) vs. f64 (CPU),
// same precision-tolerance story every prior phase's numerical-tolerance
// test already covers. inv_x/inv_y (100.0 / scale) are precomputed
// Rust-side in f64, matching resize_pixels's own formula exactly, then
// cast to f32 for upload - not recomputed from raw SCALE_X/SCALE_Y
// inside the shader itself.
const RESIZE_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> input: array<f32>;
    @group(0) @binding(1) var<storage, read_write> output: array<f32>;
    @group(0) @binding(2) var<uniform> params: vec4<u32>;

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params.x;
        let height = params.y;
        let inv_x = bitcast<f32>(params.z);
        let inv_y = bitcast<f32>(params.w);

        if (id.x >= width || id.y >= height) {
            return;
        }

        let cx = f32(width) / 2.0;
        let cy = f32(height) / 2.0;

        let src_x = cx + (f32(id.x) + 0.5 - cx) * inv_x;
        let src_y = cy + (f32(id.y) + 0.5 - cy) * inv_y;

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

struct ResizeGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_resize_pipeline(gpu: &GpuState) -> ResizeGpuPipeline {
    let shader = gpu.create_shader(RESIZE_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("resize bind group layout"),
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

    let pipeline = gpu.create_compute_pipeline("resize pipeline", &shader, "main", &[&bind_group_layout]);

    ResizeGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct ResizeFingerprint {
    source: Value,
    scale_x_bits: u64,
    scale_y_bits: u64,
}

impl ResizeFingerprint {
    fn matches(&self, other: &ResizeFingerprint) -> bool {
        self.scale_x_bits == other.scale_x_bits && self.scale_y_bits == other.scale_y_bits && value_ptr_eq(&self.source, &other.source)
    }
}

struct CompletedResizeJob {
    fingerprint: ResizeFingerprint,
    pixels: Vec<f32>,
    width: u32,
    height: u32,
}

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
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale (Rc so the wasm32 spawn_local task can share the
    // cell without requiring `self` to be 'static).
    gpu_pipeline: RefCell<Option<ResizeGpuPipeline>>,
    pending: Rc<RefCell<Option<ResizeFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedResizeJob>>>,
}

impl Resize {
    pub fn new() -> Self {
        Self {
            scale_x: 100.0,
            scale_y: 100.0,
            algorithm: ResizeAlgorithm::default(),
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Blur::dispatch_gpu` exactly in structure (see its own doc
    /// comment for the target-conditional readback rationale). `inv_x`/
    /// `inv_y` are precomputed here in f64, matching `resize_pixels`'s own
    /// formula exactly, then cast to f32 for upload - not recomputed from
    /// raw scale inside the shader itself.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: ResizeFingerprint, source: FloatImage, scale_x: f64, scale_y: f64) {
        let width = source.width;
        let height = source.height;
        let len = source.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_resize_pipeline(&gpu));
            }
        }

        let input_buffer = gpu.upload("resize input", &source.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "resize output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );

        let inv_x = (100.0 / scale_x) as f32;
        let inv_y = (100.0 / scale_y) as f32;
        let params: [u32; 4] = [width, height, inv_x.to_bits(), inv_y.to_bits()];
        let params_buffer = gpu.upload("resize params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "resize readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("resize bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("resize dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pixels = gpu.read_buffer_blocking(&readback_buffer, len);
            *self.last_gpu_result.borrow_mut() = Some(CompletedResizeJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pixels = gpu.read_buffer_async(&readback_buffer, len).await;
                *last_gpu_result.borrow_mut() = Some(CompletedResizeJob {
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

        // No MASK input exists for this operation (see metadata()'s own
        // comment) - GPU dispatch applies unconditionally when available,
        // no blanket-rule split needed. Gated on
        // `algorithm == NearestNeighbor` defensively: the shader only ever
        // implements nearest-neighbor sampling, so a future BILINEAR
        // variant must fall back to CPU until it gets its own shader
        // branch, rather than silently computing the wrong resample on GPU.
        if let Some(gpu) = ctx.gpu.clone() {
            if self.algorithm == ResizeAlgorithm::NearestNeighbor {
                let fingerprint = ResizeFingerprint {
                    source: value.clone(),
                    scale_x_bits: self.scale_x.to_bits(),
                    scale_y_bits: self.scale_y.to_bits(),
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
                    self.dispatch_gpu(gpu, fingerprint, source, self.scale_x, self.scale_y);
                }
            }
        }

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

    // --- WebGPU Phase 2 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let resize = Resize::new();
        assert!(!resize.is_live(), "no dispatch in flight yet - must not force re-execution");

        *resize.pending.borrow_mut() = Some(ResizeFingerprint {
            source: Value::Number(0.0),
            scale_x_bits: 50.0_f64.to_bits(),
            scale_y_bits: 50.0_f64.to_bits(),
        });
        assert!(resize.is_live(), "a pending GPU dispatch must force re-execution so a just-completed result gets picked up");

        *resize.pending.borrow_mut() = None;
        assert!(!resize.is_live(), "once nothing is pending, normal cross-tick caching should resume");
    }

    #[test]
    fn gpu_resize_matches_cpu_within_tolerance_once_warmed_up() {
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

        let mut cpu_resize = Resize::new();
        cpu_resize.set_parameter("SCALE_X", Value::Number(150.0)).unwrap();
        cpu_resize.set_parameter("SCALE_Y", Value::Number(60.0)).unwrap();
        let cpu_values = cpu_resize
            .execute(&context(width, height), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();
        let Value::FloatImage(cpu_result) = &cpu_values[0] else { panic!("expected a float image") };

        let mut gpu_resize = Resize::new();
        gpu_resize.set_parameter("SCALE_X", Value::Number(150.0)).unwrap();
        gpu_resize.set_parameter("SCALE_Y", Value::Number(60.0)).unwrap();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };

        // First call: no cached GPU result yet - kicks off a dispatch
        // (native resolves it synchronously inside dispatch_gpu) and
        // falls back to CPU for this tick regardless.
        let _ = gpu_resize.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        // Second call, same fingerprint (same wired Arc, same scales):
        // the now-completed GPU result is cached and used directly.
        let gpu_values = gpu_resize.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
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
