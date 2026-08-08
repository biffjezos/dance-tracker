// src/operations/transform/blur.rs
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

// GPU compute shader for BLUR's unmasked path - see
// SPECwebgpuoperations.md's Phase 0 and SPECwebgpucomputebackend-1.md's
// "the pattern every GPU-backed operation follows". Single-pass 2D window
// average (not the CPU path's separable two-pass), deliberately: per
// blur_single_pixel's own doc comment, a separable box blur's two-pass
// average is mathematically identical to the single-pass 2D window
// average over the same window, so this is a correct (if not maximally
// fast) "naive per-invocation neighbor read" first cut, exactly what the
// spec explicitly allows for Phase 0 rather than requiring workgroup-
// shared-memory tiling up front. Edge handling matches blur_single_pixel
// exactly: clamp the window to the frame, average only the pixels that
// exist (no wraparound, no out-of-frame zero-padding).
const BLUR_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> input: array<f32>;
    @group(0) @binding(1) var<storage, read_write> output: array<f32>;
    @group(0) @binding(2) var<uniform> params: vec4<u32>;

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params.x;
        let height = params.y;
        let radius = params.z;

        if (id.x >= width || id.y >= height) {
            return;
        }

        let x_start = select(id.x - radius, 0u, id.x < radius);
        let x_end = min(id.x + radius, width - 1u);
        let y_start = select(id.y - radius, 0u, id.y < radius);
        let y_end = min(id.y + radius, height - 1u);

        var sum = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        var count: f32 = 0.0;

        for (var yi = y_start; yi <= y_end; yi = yi + 1u) {
            let row = yi * width;
            for (var xi = x_start; xi <= x_end; xi = xi + 1u) {
                let idx = (row + xi) * 4u;
                sum = sum + vec4<f32>(input[idx], input[idx + 1u], input[idx + 2u], input[idx + 3u]);
                count = count + 1.0;
            }
        }

        let out_idx = (id.y * width + id.x) * 4u;
        let avg = sum / count;
        output[out_idx] = avg.x;
        output[out_idx + 1u] = avg.y;
        output[out_idx + 2u] = avg.z;
        output[out_idx + 3u] = avg.w;
    }
"#;

/// Lazily built the first time `ctx.gpu` is `Some` - see the pattern
/// spec's "gpu_pipeline: RefCell<Option<OpGpuPipeline>>". Operation-owned,
/// not shared: lives here, next to Blur, not in the `gpu` module.
struct BlurGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_blur_pipeline(gpu: &GpuState) -> BlurGpuPipeline {
    let shader = gpu.create_shader(BLUR_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("blur bind group layout"),
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

    let pipeline = gpu.create_compute_pipeline("blur pipeline", &shader, "main", &[&bind_group_layout]);

    BlurGpuPipeline { pipeline, bind_group_layout }
}

/// Captured from the *wired* `Value` (via `find_input`), before
/// `FloatImage::from_value` clones it out of its `Arc` - see the pattern
/// spec's "Fingerprint, precisely" section. Compared via `value_ptr_eq`,
/// the same function `RenderExecutor`'s own cross-tick cache uses.
#[derive(Clone)]
struct BlurFingerprint {
    source: Value,
    radius_px: u32,
}

impl BlurFingerprint {
    fn matches(&self, other: &BlurFingerprint) -> bool {
        self.radius_px == other.radius_px && value_ptr_eq(&self.source, &other.source)
    }
}

struct CompletedBlurJob {
    fingerprint: BlurFingerprint,
    pixels: Vec<f32>,
    width: u32,
    height: u32,
}

/// A simple separable box blur.
///
/// radius_px = 0 means “no blur” (identity).
/// radius_px > 0 applies a box kernel of width (2 * radius + 1).
pub struct Blur {
    pub radius_px: u32,
    // GPU-backed dispatch state - see SPECwebgpucomputebackend-1.md's "The
    // pattern every GPU-backed operation follows". Only ever consulted/
    // mutated from the unmasked path (the blanket rule in
    // SPECwebgpuoperations.md: GPU dispatch only ever replaces the
    // unmasked, full-frame path). `pending`/`last_gpu_result` are `Rc`,
    // not a bare `RefCell`, because the wasm32 async readback task
    // (`wasm_bindgen_futures::spawn_local`) needs a handle to the same
    // cell that outlives the `&self` borrow of the `execute()` call that
    // spawned it - an `Rc` clone gives it that without requiring `self`
    // itself to be `'static`.
    gpu_pipeline: RefCell<Option<BlurGpuPipeline>>,
    pending: Rc<RefCell<Option<BlurFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedBlurJob>>>,
}

impl Blur {
    pub fn new() -> Self {
        Self {
            radius_px: 0,
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
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
        pub(crate) fn blur_pixels_static(  pixels: &[f32], width: u32, height: u32, radius: u32, ) -> Vec<f32> {
            if radius == 0 {
                return pixels.to_vec();
            }

        let w = width as usize;
        let h = height as usize;
        let r = radius as usize;

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

    /// The blurred value of a single output pixel, computed directly from
    /// `pixels` rather than via the two-pass `blur_pixels` above -
    /// mathematically identical to it (not an approximation): a separable
    /// box blur's per-axis pixel count at a given output x (or y) never
    /// depends on the other axis, so the two-pass average
    /// `(1/countY) * sum_y[ (1/countX) * sum_x pixels ]` always equals the
    /// single-pass 2D window average `sum / (countX * countY)` over the
    /// same rectangular window. Used by `execute()`'s bbox-restricted path
    /// (Phase 3 of BBOX_CONVENTIONS.md) via `compute_within_bbox`, so only
    /// the pixels actually inside the relevant work area get this
    /// per-pixel recomputation instead of the full two-pass buffer.
    fn blur_single_pixel(pixels: &[f32], width: u32, height: u32, radius: u32, x: u32, y: u32) -> [f32; 4] {
        let w = width as usize;
        let h = height as usize;
        let r = radius as usize;
        let xu = x as usize;
        let yu = y as usize;

        let x_start = xu.saturating_sub(r);
        let x_end = (xu + r).min(w - 1);
        let y_start = yu.saturating_sub(r);
        let y_end = (yu + r).min(h - 1);

        let mut sum = [0f32; 4];
        let mut count = 0u32;

        for yi in y_start..=y_end {
            let row_start = yi * w * 4;
            for xi in x_start..=x_end {
                let idx = row_start + xi * 4;
                sum[0] += pixels[idx];
                sum[1] += pixels[idx + 1];
                sum[2] += pixels[idx + 2];
                sum[3] += pixels[idx + 3];
                count += 1;
            }
        }

        let inv = 1.0 / count as f32;
        [sum[0] * inv, sum[1] * inv, sum[2] * inv, sum[3] * inv]
    }

    /// BLUR's own true content-spread extent, given SOURCE's own reported
    /// box: a box blur pulls real neighboring content up to `radius_px`
    /// pixels beyond SOURCE's own non-default region (see `output_bbox`'s
    /// own doc comment), so this is SOURCE's box grown by the radius and
    /// clamped to the frame. Shared by `output_bbox()` (the reported
    /// metadata) and `execute()`'s masked path (the actual work area) so
    /// the two can never drift apart the way they once did - the masked
    /// path originally intersected against SOURCE's *raw*, un-grown box,
    /// silently skipping real blur computation in the radius-wide
    /// penumbra just outside it (caught by evaluator review).
    fn natural_bbox(&self, ctx: &Context, input_bboxes: &[(Input, Rect)]) -> Rect {
        let source_box = find_bbox(input_bboxes, Input::Source)
            .unwrap_or_else(|| Rect::full(ctx.meta.width, ctx.meta.height));

        if source_box.is_empty() {
            return Rect::empty();
        }

        source_box
            .grow(self.radius_px as i32)
            .intersect(&Rect::full(ctx.meta.width, ctx.meta.height))
    }

    /// Encodes and submits a fresh GPU dispatch for `fingerprint`, then
    /// resolves it per target - see SPECwebgpucomputebackend-1.md's
    /// "Target-conditional dispatch, not two designs". Upload/dispatch/
    /// copy are all ordinary synchronous wgpu calls on both targets (only
    /// buffer *readback* genuinely differs): native backends support a
    /// blocking read, so this resolves `last_gpu_result` directly within
    /// this same call, and `pending` is never observably left `Some`.
    /// wasm32 cannot block, so it hands the async readback to
    /// `wasm_bindgen_futures::spawn_local`, records `pending` until that
    /// resolves, and only the `Rc`-shared cells (not `self`) are captured
    /// by the spawned task.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: BlurFingerprint, source: FloatImage) {
        let width = source.width;
        let height = source.height;
        let radius = fingerprint.radius_px;
        let len = source.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_blur_pipeline(&gpu));
            }
        }

        let input_buffer = gpu.upload("blur input", &source.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "blur output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        // vec4<u32> in the shader - 16 bytes, no internal padding, so a
        // plain [u32; 4] uploads with matching layout directly (same Pod
        // array usage already relied on by gpu/mod.rs's own test).
        let params: [u32; 4] = [width, height, radius, 0];
        let params_buffer = gpu.upload("blur params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "blur readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("blur bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("blur dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pixels = gpu.read_buffer_blocking(&readback_buffer, len);
            *self.last_gpu_result.borrow_mut() = Some(CompletedBlurJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pixels = gpu.read_buffer_async(&readback_buffer, len).await;
                *last_gpu_result.borrow_mut() = Some(CompletedBlurJob {
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

    // A pending GPU dispatch must force re-execution so a just-completed
    // result actually gets picked up - see SPECwebgpucomputebackend-1.md's
    // "Required correctness detail". Without this, RenderExecutor's
    // cross-tick cache would call execute() exactly once on a static
    // upstream graph; if that one call fell back to CPU (GPU not ready
    // yet), it would never re-run to notice the GPU result arriving,
    // staying stuck on CPU forever. Native's blocking dispatch resolves
    // within the same execute() call, so `pending` is never left `Some`
    // there in practice - this only actually forces a re-tick on wasm32,
    // but the check itself is target-independent, same as the field.
    fn is_live(&self) -> bool {
        self.pending.borrow().is_some()
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        // Resolved once up front - MASK is independent of which concrete
        // Value variant SOURCE turns out to be.
        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        // Blanket rule (SPECwebgpuoperations.md): GPU dispatch only ever
        // replaces the *unmasked* full-frame path. With a MASK wired, this
        // is the exact same masked path as before this phase, byte-for-
        // byte - apply_mask below blends every pixel outside the relevant
        // region straight back to `original` anyway, so restricting the
        // actual blur compute to the intersection of MASK's own reported
        // box and this node's own natural bbox (SOURCE's box grown by
        // radius - see natural_bbox()'s own doc comment for why the
        // growth is required here, not just SOURCE's raw box) skips the
        // rest instead of running the full two-pass blur unconditionally.
        if let Some(mask) = &mask {
            let source = FloatImage::from_value(value, ctx)?;

            let natural_box = self.natural_bbox(ctx, &ctx.input_bboxes);
            let mask_box = find_bbox(&ctx.input_bboxes, Input::Mask)
                .unwrap_or_else(|| Rect::full(source.width, source.height));
            let work_area = natural_box.intersect(&mask_box);

            let radius = self.radius_px;
            let width = source.width;
            let height = source.height;
            let pixels = &source.pixels;

            let blurred = crate::graphics::compute_within_bbox(width, height, work_area, pixels, |x, y| {
                Self::blur_single_pixel(pixels, width, height, radius, x, y)
            });
            let blurred = crate::graphics::apply_mask(&source.pixels, blurred, Some(mask), width, height)?;

            return Ok(vec![Value::FloatImage(Arc::new(FloatImage { pixels: blurred, width, height }))]);
        }

        // Unmasked path: try GPU first when available and there's
        // actually something to blur (RADIUS=0 is a trivial identity -
        // not worth a dispatch).
        if let Some(gpu) = ctx.gpu.clone() {
            if self.radius_px > 0 {
                let fingerprint = BlurFingerprint { source: value.clone(), radius_px: self.radius_px };

                let cached = self.last_gpu_result.borrow().as_ref()
                    .filter(|completed| completed.fingerprint.matches(&fingerprint))
                    .map(|completed| FloatImage { pixels: completed.pixels.clone(), width: completed.width, height: completed.height });

                if let Some(result) = cached {
                    return Ok(vec![Value::FloatImage(Arc::new(result))]);
                }

                let already_pending = self.pending.borrow().as_ref().is_some_and(|p| p.matches(&fingerprint));
                if !already_pending {
                    let source = FloatImage::from_value(value, ctx)?;
                    self.dispatch_gpu(gpu, fingerprint, source);
                }
                // Fall through to the CPU path for this tick - either a
                // fresh dispatch was just kicked off, or one is already
                // in flight; neither has a result ready yet.
            }
        }

        let source = FloatImage::from_value(value, ctx)?;
        let blurred = Self::blur_pixels_static(&source.pixels, source.width, source.height, self.radius_px);

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
        self.natural_bbox(ctx, input_bboxes)
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

    #[test]
    fn consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one() {
        // BBOX_CONVENTIONS.md's Phase 3 consume-equivalence invariant:
        // restricting compute to a bbox must never change the final
        // (apply_mask-blended) output versus running unrestricted. Same
        // Source/Mask pixel data both times - only the *reported* bbox
        // metadata passed via ctx.input_bboxes differs.
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(1.0)).unwrap();

        // 6x1: distinct source pixels so blur produces distinct values
        // per output pixel; mask is fully opaque (weight 1) only at
        // x=2..4, zero elsewhere.
        let source = image(
            (0..6).flat_map(|n| [n * 10, n * 10 + 1, n * 10 + 2, 255]).collect(),
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
                (Input::Source, crate::compositor::bbox::Rect::full(6, 1)),
                (Input::Mask, crate::compositor::bbox::Rect { x0: 2, y0: 0, x1: 4, y1: 1 }),
            ],
            ..context(6, 1)
        };
        let ctx_full_frame = context(6, 1); // input_bboxes empty -> falls back to Rect::full everywhere

        let restricted = blur.execute(&ctx_with_real_box, &inputs).unwrap();
        let unrestricted = blur.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box() {
        // Regression (evaluator-caught): the masked work_area must be
        // intersected against SOURCE's box *grown by radius*, not
        // SOURCE's raw reported box - a box blur pulls real content from
        // up to `radius_px` beyond SOURCE's own non-default region. Using
        // the raw box silently skipped real computation in that
        // radius-wide penumbra whenever SOURCE itself reported a
        // sub-frame box (e.g. fed from a RESIZE/MOVE chain).
        //
        // 10x1 frame: SOURCE is genuinely [0,0,0,0] outside [3,7), real
        // content [100,100,100,255] inside it (a valid precondition -
        // SOURCE's reported box actually matches where its real content
        // is). RADIUS=2, MASK fully opaque everywhere - so every pixel's
        // blur must actually be computed, including x=1, which sits in
        // the radius-2 penumbra just outside [3,7) and must show real
        // blurred content pulled in from x=3's neighborhood, not be left
        // as raw transparent.
        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(2.0)).unwrap();

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
                (Input::Source, crate::compositor::bbox::Rect { x0: 3, y0: 0, x1: 7, y1: 1 }),
                (Input::Mask, crate::compositor::bbox::Rect::full(10, 1)),
            ],
            ..context(10, 1)
        };
        let ctx_full_frame = context(10, 1); // input_bboxes empty -> full-frame fallback, ground truth

        let restricted = blur.execute(&ctx_with_real_source_box, &inputs).unwrap();
        let unrestricted = blur.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(
            as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]),
            "SOURCE's radius-wide penumbra must still be computed, not left as raw transparent"
        );

        // Directly pin down the specific pixel the bug used to get wrong.
        let restricted_pixels = as_u8_pixels(&restricted[0]);
        assert_ne!(&restricted_pixels[4..8], &[0, 0, 0, 0], "x=1 sits in the radius-2 penumbra and must show real blurred content, not raw transparent");
    }

    #[test]
    fn a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one() {
        use crate::graphics::mask::{reset_pixels_computed, take_pixels_computed};

        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(1.0)).unwrap();

        let source = image((0..16).flat_map(|n| [n, n, n, 255]).collect(), 4, 4);
        let mask = image(vec![255; 4 * 4 * 4], 4, 4); // fully opaque everywhere - doesn't affect which pixels get computed, only the reported box does

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let small_box_ctx = Context {
            input_bboxes: vec![
                (Input::Source, crate::compositor::bbox::Rect::full(4, 4)),
                (Input::Mask, crate::compositor::bbox::Rect { x0: 1, y0: 1, x1: 2, y1: 2 }),
            ],
            ..context(4, 4)
        };
        reset_pixels_computed();
        blur.execute(&small_box_ctx, &inputs).unwrap();
        let small_box_pixels = take_pixels_computed().expect("BLUR with a wired MASK must record a pixel count");

        let full_frame_ctx = context(4, 4); // no reported Mask box -> falls back to full-frame
        reset_pixels_computed();
        blur.execute(&full_frame_ctx, &inputs).unwrap();
        let full_frame_pixels = take_pixels_computed().expect("BLUR with a wired MASK must record a pixel count");

        assert_eq!(small_box_pixels, 1, "a 1x1 mask box must compute exactly one pixel");
        assert_eq!(full_frame_pixels, 16, "a full-frame mask box must compute every pixel");
        assert!(small_box_pixels < full_frame_pixels, "a smaller mask bbox must do strictly less work");
    }

    #[test]
    fn checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off() {
        // AC3-style graph-level integration test: a geometric mask
        // (CHECKERBOARD -> RESIZE -> MOVE, the same motivating pipeline
        // from the MOVE spec) wired as BLUR's own MASK, confirming
        // end-to-end pixel-identical output whether real (bbox-restricted,
        // "on") boxes are threaded through the graph or not ("off",
        // simulating pre-Phase-3 always-full-frame behaviour).
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

        let mut blur = Blur::new();
        blur.set_parameter("RADIUS", Value::Number(1.0)).unwrap();
        let blur_id = graph.add_node(Box::new(blur));
        graph.connect(blur_id, Input::Source, source_id).unwrap();
        graph.connect(blur_id, Input::Mask, move_id).unwrap();

        graph.validate().expect("the wired pipeline is valid");

        let ctx = context(4, 4);

        // "On": through the real graph/RenderExecutor, which computes and
        // threads real (non-full-frame) boxes from CHECKERBOARD -> RESIZE
        // -> MOVE into BLUR's own ctx.input_bboxes.
        let on_values = RenderExecutor::new().execute(&graph, blur_id, &ctx).unwrap();
        let on_pixels = as_u8_pixels(&on_values[0]);

        // "Off": call BLUR's own execute() directly with the same resolved
        // Source/Mask Values (fetched by evaluating those same graph nodes)
        // but no bbox info at all - ctx.input_bboxes stays empty, so
        // find_bbox falls back to full-frame everywhere, exactly
        // pre-Phase-3 behaviour.
        let source_value = PreviewExecutor::default().execute(&graph, source_id, &ctx).unwrap().into_iter().next().unwrap();
        let mask_value = PreviewExecutor::default().execute(&graph, move_id, &ctx).unwrap().into_iter().next().unwrap();

        let blur_off = Blur { radius_px: 1, ..Blur::new() };
        let off_values = blur_off.execute(&ctx, &[
            (Input::Source, source_value),
            (Input::Mask, mask_value),
        ]).unwrap();
        let off_pixels = as_u8_pixels(&off_values[0]);

        assert_eq!(on_pixels, off_pixels, "bbox consumption on vs off must produce pixel-identical output");
    }

    // --- WebGPU Phase 0 (SPECwebgpuoperations.md / SPECwebgpucomputebackend-1.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        // Direct unit test of is_live()'s own read-through logic (per
        // SPECwebgpucomputebackend-1.md's "Required correctness detail"),
        // independent of whether a real dispatch is actually in flight -
        // native's blocking dispatch resolves synchronously within
        // dispatch_gpu() itself, so `pending` is never observably `Some`
        // there in a real run; this regression coverage is what every
        // per-operation phase is required to have regardless.
        let blur = Blur::new();
        assert!(!blur.is_live(), "no dispatch in flight yet - must not force re-execution");

        *blur.pending.borrow_mut() = Some(BlurFingerprint { source: Value::Number(0.0), radius_px: 3 });
        assert!(blur.is_live(), "a pending GPU dispatch must force re-execution so a just-completed result gets picked up");

        *blur.pending.borrow_mut() = None;
        assert!(!blur.is_live(), "once nothing is pending, normal cross-tick caching should resume");
    }

    #[test]
    fn gpu_blur_matches_cpu_blur_within_tolerance_once_warmed_up() {
        // Numerical-tolerance GPU-vs-CPU test - see
        // SPECwebgpucomputebackend-1.md's "Numerical tolerance". Skips
        // gracefully with no adapter available, the same precedent
        // gpu/mod.rs's own test already establishes for this sandbox/CI
        // environment.
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

        let mut cpu_blur = Blur::new();
        cpu_blur.set_parameter("RADIUS", Value::Number(2.0)).unwrap();
        let cpu_values = cpu_blur
            .execute(&context(width, height), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();
        let Value::FloatImage(cpu_result) = &cpu_values[0] else { panic!("expected a float image") };

        let mut gpu_blur = Blur::new();
        gpu_blur.set_parameter("RADIUS", Value::Number(2.0)).unwrap();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };

        // First call: no cached GPU result yet - kicks off a dispatch
        // (native resolves it synchronously inside dispatch_gpu) and
        // falls back to CPU for this tick regardless.
        let _ = gpu_blur.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        // Second call, same fingerprint (same wired Arc, same radius):
        // the now-completed GPU result is cached and used directly.
        let gpu_values = gpu_blur.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
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