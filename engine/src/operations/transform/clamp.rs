// src/operations/transform/clamp.rs
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
use crate::graphics::{FloatImage, ImageFormat, U8Image};
use crate::gpu::GpuState;

// GPU compute shader - SPECwebgpuoperations.md's Phase 1.1. CLAMP has no
// MASK input at all, so GPU dispatch applies unconditionally when
// available, same as RGB_TO_HSV. Unlike every other Phase 1.1 operation,
// CLAMP's own output is a quantized U8Image, not a FloatImage - the
// shader still only computes the clamped float per channel (matching
// `to_image_clamped`'s own `c.clamp(min, max)` step exactly); the
// `* 255.0` round-to-u8 quantization happens once, CPU-side, immediately
// after readback (see `dispatch_gpu`'s own comment) - `gpu/mod.rs`'s
// readback helpers only ever read back `Vec<f32>`, so there's no GPU-side
// path to u8 without a new readback helper this phase doesn't need.
const CLAMP_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> input: array<f32>;
    @group(0) @binding(1) var<storage, read_write> output: array<f32>;
    @group(0) @binding(2) var<uniform> params: vec4<u32>;

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params.x;
        let height = params.y;
        let min_v = bitcast<f32>(params.z);
        let max_v = bitcast<f32>(params.w);

        if (id.x >= width || id.y >= height) {
            return;
        }

        let idx = (id.y * width + id.x) * 4u;
        output[idx] = clamp(input[idx], min_v, max_v);
        output[idx + 1u] = clamp(input[idx + 1u], min_v, max_v);
        output[idx + 2u] = clamp(input[idx + 2u], min_v, max_v);
        output[idx + 3u] = clamp(input[idx + 3u], min_v, max_v);
    }
"#;

struct ClampGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_clamp_pipeline(gpu: &GpuState) -> ClampGpuPipeline {
    let shader = gpu.create_shader(CLAMP_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("clamp bind group layout"),
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

    let pipeline = gpu.create_compute_pipeline("clamp pipeline", &shader, "main", &[&bind_group_layout]);

    ClampGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct ClampFingerprint {
    source: Value,
    min_bits: u64,
    max_bits: u64,
}

impl ClampFingerprint {
    fn matches(&self, other: &ClampFingerprint) -> bool {
        self.min_bits == other.min_bits && self.max_bits == other.max_bits && value_ptr_eq(&self.source, &other.source)
    }
}

struct CompletedClampJob {
    fingerprint: ClampFingerprint,
    // Already quantized to u8 (see the shader's own doc comment) - this
    // is the one CompletedJob in Phase 1.1 that doesn't hold Vec<f32>.
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

/// Explicit, deliberate step from an unbounded FloatImage back down to a
/// normal bounded Image - the one place an out-of-gamut value actually
/// gets thrown away. MIN/MAX default to 0.0/1.0 (the standard "bring back
/// into gamut" case) but are adjustable for a creative clip (crush
/// blacks, clip highlights early) - applied uniformly regardless of
/// whether the input happens to already be bounded, so a narrowed range
/// still does something to a normal Image/Frame/Video, not just a
/// FloatImage. With the default 0.0/1.0 range, clamping an
/// already-bounded input is a true no-op, which is what makes CLAMP
/// always safe to insert everywhere.
pub struct Clamp {
    pub min: f64,
    pub max: f64,
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale.
    gpu_pipeline: RefCell<Option<ClampGpuPipeline>>,
    pending: Rc<RefCell<Option<ClampFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedClampJob>>>,
}

impl Clamp {
    pub fn new() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Blur::dispatch_gpu` in structure, with one difference:
    /// the read-back `Vec<f32>` is quantized to `Vec<u8>` immediately
    /// (matching `to_image_clamped`'s own `(c * 255.0).round() as u8` -
    /// the clamp itself already happened GPU-side, so no re-clamping here)
    /// before being stored, since CLAMP's own output type is U8Image.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: ClampFingerprint, source: FloatImage) {
        let width = source.width;
        let height = source.height;
        let len = source.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_clamp_pipeline(&gpu));
            }
        }

        let input_buffer = gpu.upload("clamp input", &source.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "clamp output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let min_bits = (f64::from_bits(fingerprint.min_bits) as f32).to_bits();
        let max_bits = (f64::from_bits(fingerprint.max_bits) as f32).to_bits();
        let params: [u32; 4] = [width, height, min_bits, max_bits];
        let params_buffer = gpu.upload("clamp params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "clamp readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("clamp bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("clamp dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let raw = gpu.read_buffer_blocking(&readback_buffer, len);
            let pixels: Vec<u8> = raw.iter().map(|c| (c * 255.0).round() as u8).collect();
            *self.last_gpu_result.borrow_mut() = Some(CompletedClampJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let raw = gpu.read_buffer_async(&readback_buffer, len).await;
                let pixels: Vec<u8> = raw.iter().map(|c| (c * 255.0).round() as u8).collect();
                *last_gpu_result.borrow_mut() = Some(CompletedClampJob {
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

impl Default for Clamp {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Clamp {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "clamp",
            menu: "TRANSFORM",
            label: "CLAMP",
            action: None,
            ui_action: None,
            create_node: Some("clamp"),
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
            display_name: "Clamp",
            category: OperationCategory::Color,
            inputs: vec![InputDescriptor { kind: Input::Source, accepts: PIXEL_KINDS }],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "MIN",
                kind: ParameterKind::Number { step: 0.01, min: Some(-10.0), max: Some(10.0) },
                group: None,
            },
            ParameterDescriptor {
                name: "MAX",
                kind: ParameterKind::Number { step: 0.01, min: Some(-10.0), max: Some(10.0) },
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "MIN" => Some(Value::Number(self.min)),
            "MAX" => Some(Value::Number(self.max)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("MIN", Value::Number(v)) => {
                if v >= self.max {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.min = v;
                Ok(())
            }
            ("MAX", Value::Number(v)) => {
                if v <= self.min {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.max = v;
                Ok(())
            }
            (name, _) => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    // See blur.rs's identical override for the full rationale.
    fn is_live(&self) -> bool {
        self.pending.borrow().is_some()
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        // No MASK input exists for this operation - GPU dispatch applies
        // unconditionally when available.
        if let Some(gpu) = ctx.gpu.clone() {
            let fingerprint = ClampFingerprint { source: value.clone(), min_bits: self.min.to_bits(), max_bits: self.max.to_bits() };

            let cached = self.last_gpu_result.borrow().as_ref()
                .filter(|completed| completed.fingerprint.matches(&fingerprint))
                .map(|completed| U8Image { pixels: completed.pixels.clone(), width: completed.width, height: completed.height, format: ImageFormat::Rgba8 });

            if let Some(result) = cached {
                return Ok(vec![Value::Image(Arc::new(result))]);
            }

            let already_pending = self.pending.borrow().as_ref().is_some_and(|p| p.matches(&fingerprint));
            if !already_pending {
                let source = FloatImage::from_value(value, ctx)?;
                self.dispatch_gpu(gpu, fingerprint, source);
            }
        }

        let source = FloatImage::from_value(value, ctx)?;
        let image = source.to_image_clamped(self.min as f32, self.max as f32);

        Ok(vec![Value::Image(Arc::new(image))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Clamp::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn clamps_an_out_of_gamut_float_image_to_the_default_0_1_range() {
        let clamp = Clamp::new();
        let float_image = FloatImage { pixels: vec![1.5, -0.2, 0.5, 1.0], width: 1, height: 1 };

        let values = clamp
            .execute(&context(1, 1), &[(Input::Source, Value::FloatImage(Arc::new(float_image)))])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, vec![255, 0, 128, 255]),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn a_custom_range_can_crush_or_extend_the_clip_points() {
        let mut clamp = Clamp::new();
        clamp.set_parameter("MIN", Value::Number(0.2)).unwrap();
        clamp.set_parameter("MAX", Value::Number(0.8)).unwrap();
        let float_image = FloatImage { pixels: vec![0.0, 1.0, 0.5, 1.0], width: 1, height: 1 };

        let values = clamp
            .execute(&context(1, 1), &[(Input::Source, Value::FloatImage(Arc::new(float_image)))])
            .unwrap();

        // Alpha is clamped uniformly with RGB (same convention as ADD/
        // SUBTRACT/MULTIPLY/INVERT treating all 4 channels the same way) -
        // the input's 1.0 alpha clips down to MAX (0.8) same as any
        // other channel would.
        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, vec![51, 204, 128, 204]),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn an_already_bounded_image_is_unchanged_by_the_default_range() {
        let clamp = Clamp::new();
        let image = Arc::new(crate::graphics::U8Image {
            pixels: vec![10, 20, 30, 255], width: 1, height: 1, format: crate::graphics::ImageFormat::Rgba8,
        });

        let values = clamp
            .execute(&context(1, 1), &[(Input::Source, Value::Image(image.clone()))])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, image.pixels),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn a_custom_range_crushes_an_already_bounded_image_too() {
        // Regression: CLAMP used to special-case an already-bounded Image
        // as a free pass-through, which meant a narrowed MIN/MAX silently
        // did nothing to it - only FloatImage input was ever actually
        // clamped. CLAMP must apply MIN/MAX uniformly regardless of input.
        let mut clamp = Clamp::new();
        clamp.set_parameter("MIN", Value::Number(0.5)).unwrap();
        let image = Arc::new(crate::graphics::U8Image {
            pixels: vec![0, 255, 0, 255], width: 1, height: 1, format: crate::graphics::ImageFormat::Rgba8,
        });

        let values = clamp
            .execute(&context(1, 1), &[(Input::Source, Value::Image(image))])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels[0], 128), // 0 crushed up to MIN (0.5 * 255, rounded)
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn set_parameter_rejects_a_min_that_would_cross_max() {
        let mut clamp = Clamp::new();
        let err = clamp.set_parameter("MIN", Value::Number(2.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn set_parameter_rejects_a_max_that_would_cross_min() {
        let mut clamp = Clamp::new();
        let err = clamp.set_parameter("MAX", Value::Number(-1.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn an_unwired_clamp_shows_the_missing_placeholder() {
        let clamp = Clamp::new();
        let values = clamp.execute(&context(2, 2), &[]).unwrap();
        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels.len(), (2 * 2 * 4) as usize),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    // --- WebGPU Phase 1.1 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let clamp = Clamp::new();
        assert!(!clamp.is_live());

        *clamp.pending.borrow_mut() = Some(ClampFingerprint { source: Value::Number(0.0), min_bits: 0.0f64.to_bits(), max_bits: 1.0f64.to_bits() });
        assert!(clamp.is_live());

        *clamp.pending.borrow_mut() = None;
        assert!(!clamp.is_live());
    }

    #[test]
    fn gpu_clamp_matches_cpu_within_tolerance_once_warmed_up() {
        let Ok(gpu) = pollster::block_on(crate::gpu::GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };
        let gpu = Arc::new(gpu);

        let width = 5;
        let height = 3;
        let pixels: Vec<u8> = (0..(width * height))
            .flat_map(|n| {
                let v = ((n * 47) % 256) as u8;
                [v, v.wrapping_add(60), v.wrapping_add(180), 255]
            })
            .collect();
        let input = Arc::new(crate::graphics::U8Image {
            pixels,
            width,
            height,
            format: crate::graphics::ImageFormat::Rgba8,
        });

        let mut cpu_clamp = Clamp::new();
        cpu_clamp.set_parameter("MIN", Value::Number(0.2)).unwrap();
        cpu_clamp.set_parameter("MAX", Value::Number(0.8)).unwrap();
        let cpu_values = cpu_clamp
            .execute(&context(width, height), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();
        let Value::Image(cpu_result) = &cpu_values[0] else { panic!("expected an image") };

        let mut gpu_clamp = Clamp::new();
        gpu_clamp.set_parameter("MIN", Value::Number(0.2)).unwrap();
        gpu_clamp.set_parameter("MAX", Value::Number(0.8)).unwrap();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };
        let _ = gpu_clamp.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        let gpu_values = gpu_clamp.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        let Value::Image(gpu_result) = &gpu_values[0] else { panic!("expected an image") };

        assert_eq!(cpu_result.pixels, gpu_result.pixels);
    }
}
