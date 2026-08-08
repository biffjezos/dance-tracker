// src/operations/transform/rgb_to_hsv.rs
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    input::find_input,
    Operation,
    OperationDescriptor,
    metadata::{InputDescriptor, OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind, PIXEL_KINDS},
    value::value_ptr_eq,
    Value,
};
use crate::graphics::{Color, FloatImage};
use crate::gpu::GpuState;

// GPU compute shader - SPECwebgpuoperations.md's Phase 1.1. RGB_TO_HSV has
// no MASK input at all (see metadata()'s own comment), so unlike BLUR
// there is no masked/unmasked split - GPU dispatch, when available,
// applies unconditionally. Ported directly from Color::to_hsv(), with two
// differences forced by WGSL: `rem_euclid` doesn't exist, so the hue's
// wraparound is done by hand (`raw - 6.0 * floor(raw / 6.0)`, the
// standard floor-mod identity for a non-negative result); and the
// max==0.0 / delta==0.0 guards use real `if` branches (short-circuiting
// control flow), not `select()`, specifically to avoid ever evaluating a
// division by a possibly-zero denominator at all - `select()` evaluates
// both of its value arguments unconditionally, which BLUR's Phase 0 only
// relied on for a discarded-but-still-well-defined unsigned wraparound,
// not a division.
const RGB_TO_HSV_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> input: array<f32>;
    @group(0) @binding(1) var<storage, read_write> output: array<f32>;
    @group(0) @binding(2) var<uniform> params: vec4<u32>;

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params.x;
        let height = params.y;

        if (id.x >= width || id.y >= height) {
            return;
        }

        let idx = (id.y * width + id.x) * 4u;
        let r = input[idx];
        let g = input[idx + 1u];
        let b = input[idx + 2u];
        let a = input[idx + 3u];

        let max_c = max(r, max(g, b));
        let min_c = min(r, min(g, b));
        let delta = max_c - min_c;

        let v = max_c;
        var s: f32 = 0.0;
        if (max_c != 0.0) {
            s = delta / max_c;
        }

        var h: f32 = 0.0;
        if (delta != 0.0) {
            if (max_c == r) {
                let raw = (g - b) / delta;
                h = 60.0 * (raw - 6.0 * floor(raw / 6.0));
            } else if (max_c == g) {
                h = 60.0 * ((b - r) / delta + 2.0);
            } else {
                h = 60.0 * ((r - g) / delta + 4.0);
            }
        }

        output[idx] = h / 360.0;
        output[idx + 1u] = s;
        output[idx + 2u] = v;
        output[idx + 3u] = a;
    }
"#;

struct RgbToHsvGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_rgb_to_hsv_pipeline(gpu: &GpuState) -> RgbToHsvGpuPipeline {
    let shader = gpu.create_shader(RGB_TO_HSV_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("rgb_to_hsv bind group layout"),
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

    let pipeline = gpu.create_compute_pipeline("rgb_to_hsv pipeline", &shader, "main", &[&bind_group_layout]);

    RgbToHsvGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct RgbToHsvFingerprint {
    source: Value,
    // Included even though only one ColorFormat variant exists today -
    // see ColorFormat's own doc comment on why a second variant is
    // expected eventually. Without this, a future format change on an
    // otherwise-unchanged SOURCE would silently keep serving a stale
    // cached GPU result computed under the old format.
    format: ColorFormat,
}

impl RgbToHsvFingerprint {
    fn matches(&self, other: &RgbToHsvFingerprint) -> bool {
        self.format == other.format && value_ptr_eq(&self.source, &other.source)
    }
}

struct CompletedRgbToHsvJob {
    fingerprint: RgbToHsvFingerprint,
    pixels: Vec<f32>,
    width: u32,
    height: u32,
}

/// Target packed representation. Only HSV exists today - the enum and the
/// single-entry options list both exist already so adding another color
/// space later (e.g. YUV) is just a new match arm, not a parameter shape
/// change (same pattern as RESIZE's ALGORITHM).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorFormat {
    Hsv,
}

pub const COLOR_FORMATS: &[&str] = &["HSV"];

impl Default for ColorFormat {
    fn default() -> Self {
        ColorFormat::Hsv
    }
}

impl ColorFormat {
    pub fn to_str(&self) -> &'static str {
        match self {
            ColorFormat::Hsv => "HSV",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "HSV" => Some(ColorFormat::Hsv),
            _ => None,
        }
    }
}

/// Converts each pixel's RGB into a packed representation of the chosen
/// color space; alpha always passes through unchanged. For HSV: hue
/// (0..360 degrees, normalized to 0.0..1.0) packed into the red channel,
/// saturation into green, value into blue. The output isn't meant for
/// display - it's data for a downstream operation (HUE KEY) to read.
pub struct RgbToHsv {
    pub format: ColorFormat,
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale (Rc so the wasm32 spawn_local task can share the
    // cell without requiring `self` to be 'static).
    gpu_pipeline: RefCell<Option<RgbToHsvGpuPipeline>>,
    pending: Rc<RefCell<Option<RgbToHsvFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedRgbToHsvJob>>>,
}

impl RgbToHsv {
    pub fn new() -> Self {
        Self {
            format: ColorFormat::default(),
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Blur::dispatch_gpu` exactly in structure (see its own doc
    /// comment for the target-conditional readback rationale) - the
    /// pattern spec's "shape is identical enough that writing the first
    /// one and copying its structure for the rest is the appropriate
    /// level of reuse."
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: RgbToHsvFingerprint, source: FloatImage) {
        let width = source.width;
        let height = source.height;
        let len = source.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_rgb_to_hsv_pipeline(&gpu));
            }
        }

        let input_buffer = gpu.upload("rgb_to_hsv input", &source.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "rgb_to_hsv output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let params: [u32; 4] = [width, height, 0, 0];
        let params_buffer = gpu.upload("rgb_to_hsv params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "rgb_to_hsv readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("rgb_to_hsv bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("rgb_to_hsv dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pixels = gpu.read_buffer_blocking(&readback_buffer, len);
            *self.last_gpu_result.borrow_mut() = Some(CompletedRgbToHsvJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pixels = gpu.read_buffer_async(&readback_buffer, len).await;
                *last_gpu_result.borrow_mut() = Some(CompletedRgbToHsvJob {
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

    pub fn convert_pixels(pixels: &[f32], format: ColorFormat) -> Vec<f32> {
        let mut output = vec![0f32; pixels.len()];

        for (source, target) in pixels.chunks_exact(4).zip(output.chunks_exact_mut(4)) {
            match format {
                ColorFormat::Hsv => {
                    let color = Color {
                        r: source[0],
                        g: source[1],
                        b: source[2],
                        a: 1.0,
                    };
                    let (h, s, v) = color.to_hsv();

                    target[0] = (h / 360.0) as f32;
                    target[1] = s as f32;
                    target[2] = v as f32;
                    target[3] = source[3];
                }
            }
        }

        output
    }
}

impl Default for RgbToHsv {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for RgbToHsv {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "rgb_to_hsv",
            menu: "TRANSFORM",
            label: "RGB TO HSV",
            action: None,
            ui_action: None,
            create_node: Some("rgb_to_hsv"),
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
            display_name: "RGB to HSV",
            category: OperationCategory::Color,
            // Deliberately no Input::Mask: blending raw RGB against
            // HSV-packed values pixel-by-pixel has no meaningful result -
            // there's no "partially converted", only converted or not.
            inputs: vec![InputDescriptor { kind: Input::Source, accepts: PIXEL_KINDS }],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![ParameterDescriptor {
            name: "FORMAT",
            kind: ParameterKind::Enum(COLOR_FORMATS),
            group: None,
        }]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "FORMAT" => Some(Value::Text(self.format.to_str().to_string())),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("FORMAT", Value::Text(s)) => {
                self.format = ColorFormat::from_str(&s)
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

        // No MASK input exists for this operation - GPU dispatch applies
        // unconditionally when available, no blanket-rule split needed.
        // Gated on `format == Hsv` defensively: the shader only ever
        // implements HSV math, so a future second ColorFormat variant
        // must fall back to CPU until it gets its own shader branch,
        // rather than silently computing the wrong conversion on GPU.
        if let Some(gpu) = ctx.gpu.clone() {
            if self.format == ColorFormat::Hsv {
                let fingerprint = RgbToHsvFingerprint { source: value.clone(), format: self.format };

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
            }
        }

        let source = FloatImage::from_value(value, ctx)?;

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: Self::convert_pixels(&source.pixels, self.format),
            width: source.width,
            height: source.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(RgbToHsv::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn assert_close(actual: &[f32], expected: &[f32]) {
        for (a, e) in actual.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 0.01, "expected {:?}, got {:?}", expected, actual);
        }
    }

    #[test]
    fn pure_green_packs_to_the_expected_normalized_hsv() {
        // Green: H=120/360 ~= 0.333, S=1.0, V=1.0.
        let out = RgbToHsv::convert_pixels(&[0.0, 1.0, 0.0, 1.0], ColorFormat::Hsv);
        assert_close(&out, &[0.333, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn black_packs_to_zero_hue_and_saturation() {
        let out = RgbToHsv::convert_pixels(&[0.0, 0.0, 0.0, 1.0], ColorFormat::Hsv);
        assert_close(&out, &[0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn alpha_always_passes_through_unchanged() {
        let out = RgbToHsv::convert_pixels(&[0.0, 1.0, 0.0, 0.537], ColorFormat::Hsv);
        assert!((out[3] - 0.537).abs() < 0.001);
    }

    #[test]
    fn unconnected_rgb_to_hsv_produces_the_missing_placeholder() {
        let node = RgbToHsv::new();
        let values = node.execute(&context(2, 1), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => {
                assert_eq!(out.width, 2);
                assert_eq!(out.height, 1);
            }
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn wired_source_is_converted_through_the_graph() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor};
        use crate::operations::sources::ImageSource;

        let mut graph = Graph::new(1, 1);
        let mut source = ImageSource::new();
        source.set_image(image(vec![0, 255, 0, 255], 1, 1));
        let source_id = graph.add_node(Box::new(source));

        let node_id = graph.add_node(Box::new(RgbToHsv::new()));
        graph.connect(node_id, Input::Source, source_id).unwrap();

        let values = PreviewExecutor::default()
            .execute(&graph, node_id, &context(1, 1))
            .unwrap();

        match &values[0] {
            Value::FloatImage(out) => assert_close(&out.pixels, &[0.333, 1.0, 1.0, 1.0]),
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    // --- WebGPU Phase 1.1 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let node = RgbToHsv::new();
        assert!(!node.is_live());

        *node.pending.borrow_mut() = Some(RgbToHsvFingerprint { source: Value::Number(0.0), format: ColorFormat::Hsv });
        assert!(node.is_live());

        *node.pending.borrow_mut() = None;
        assert!(!node.is_live());
    }

    #[test]
    fn gpu_rgb_to_hsv_matches_cpu_within_tolerance_once_warmed_up() {
        let Ok(gpu) = pollster::block_on(crate::gpu::GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };
        let gpu = Arc::new(gpu);

        let width = 5;
        let height = 4;
        let pixels: Vec<u8> = (0..(width * height))
            .flat_map(|n| {
                let r = ((n * 53) % 256) as u8;
                let g = ((n * 97) % 256) as u8;
                let b = ((n * 31) % 256) as u8;
                [r, g, b, 255]
            })
            .collect();
        let input = image(pixels, width, height);

        let cpu_node = RgbToHsv::new();
        let cpu_values = cpu_node
            .execute(&context(width, height), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();
        let Value::FloatImage(cpu_result) = &cpu_values[0] else { panic!("expected a float image") };

        let gpu_node = RgbToHsv::new();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };

        let _ = gpu_node.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        let gpu_values = gpu_node.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        let Value::FloatImage(gpu_result) = &gpu_values[0] else { panic!("expected a float image") };

        assert_eq!(cpu_result.pixels.len(), gpu_result.pixels.len());
        for (index, (cpu_px, gpu_px)) in cpu_result.pixels.iter().zip(gpu_result.pixels.iter()).enumerate() {
            assert!((cpu_px - gpu_px).abs() < 1e-4, "channel {}: cpu={}, gpu={}", index, cpu_px, gpu_px);
        }
    }

    #[test]
    fn gpu_rgb_to_hsv_matches_cpu_for_out_of_gamut_negative_channels() {
        // Regression for RFC-003: the GPU saturation guard must match
        // Color::to_hsv()'s CPU guard (`max == 0.0`) exactly - it used to
        // read `max_c > 0.0`, which silently zeroed saturation for any
        // negative max_c instead of computing the real (correct) value.
        // A u8-sourced image (0..255) can never produce a negative
        // channel, so this needs a FloatImage built directly to actually
        // reach the diverging branch - the tolerance test above
        // structurally cannot, which is why RFC-003's bug wasn't caught
        // by it.
        let Ok(gpu) = pollster::block_on(crate::gpu::GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };
        let gpu = Arc::new(gpu);

        // Same worked example as RFC-003: max_c = -0.2 (negative),
        // delta = 0.6, so the CPU reference's real answer is -3.0.
        let pixels = vec![-0.2, -0.5, -0.8, 1.0, -0.1, -0.3, -0.05, 1.0];
        let source = Arc::new(FloatImage { pixels: pixels.clone(), width: 2, height: 1 });

        let cpu_result = RgbToHsv::convert_pixels(&pixels, ColorFormat::Hsv);

        let gpu_node = RgbToHsv::new();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(2, 1) };
        let _ = gpu_node.execute(&gpu_ctx, &[(Input::Source, Value::FloatImage(source.clone()))]).unwrap();
        let gpu_values = gpu_node.execute(&gpu_ctx, &[(Input::Source, Value::FloatImage(source))]).unwrap();
        let Value::FloatImage(gpu_result) = &gpu_values[0] else { panic!("expected a float image") };

        for (index, (cpu_px, gpu_px)) in cpu_result.iter().zip(gpu_result.pixels.iter()).enumerate() {
            assert!((cpu_px - gpu_px).abs() < 1e-4, "channel {}: cpu={}, gpu={}", index, cpu_px, gpu_px);
        }

        // Directly pin the specific value the bug got wrong, not just
        // GPU-vs-CPU agreement (both could theoretically agree on a wrong
        // answer) - the saturation channel must be the real negative
        // value, not silently zeroed.
        assert!((cpu_result[1] - (-3.0)).abs() < 1e-4, "expected CPU reference saturation -3.0, got {}", cpu_result[1]);
        assert!(
            (gpu_result.pixels[1] - (-3.0)).abs() < 1e-4,
            "expected GPU saturation -3.0 matching CPU, got {} - this is exactly RFC-003's bug if it reads 0.0",
            gpu_result.pixels[1]
        );
    }
}
