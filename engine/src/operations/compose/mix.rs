// src/operations/compose/mix.rs
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
use crate::graphics::FloatImage;
use crate::gpu::GpuState;

// GPU compute shader - SPECwebgpuoperations.md's Phase 1.3. MIX has no
// MASK input at all (see metadata() - same shape as RGB_TO_HSV/
// CHECKERBOARD in that regard), so GPU dispatch applies unconditionally
// when available, no blanket-rule split needed. One extra uniform value
// beyond width/height: AMOUNT, bit-packed the same way BLUR's
// MIN/MAX-style params are.
const MIX_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> foreground: array<f32>;
    @group(0) @binding(1) var<storage, read> background: array<f32>;
    @group(0) @binding(2) var<storage, read_write> output: array<f32>;
    @group(0) @binding(3) var<uniform> params: vec4<u32>;

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params.x;
        let height = params.y;
        let amount = bitcast<f32>(params.z);

        if (id.x >= width || id.y >= height) {
            return;
        }

        let idx = (id.y * width + id.x) * 4u;
        output[idx] = foreground[idx] * (1.0 - amount) + background[idx] * amount;
        output[idx + 1u] = foreground[idx + 1u] * (1.0 - amount) + background[idx + 1u] * amount;
        output[idx + 2u] = foreground[idx + 2u] * (1.0 - amount) + background[idx + 2u] * amount;
        output[idx + 3u] = foreground[idx + 3u] * (1.0 - amount) + background[idx + 3u] * amount;
    }
"#;

struct MixGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_mix_pipeline(gpu: &GpuState) -> MixGpuPipeline {
    let shader = gpu.create_shader(MIX_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mix bind group layout"),
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
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
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

    let pipeline = gpu.create_compute_pipeline("mix pipeline", &shader, "main", &[&bind_group_layout]);

    MixGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct MixFingerprint {
    foreground: Value,
    background: Value,
    amount_bits: u64,
}

impl MixFingerprint {
    fn matches(&self, other: &MixFingerprint) -> bool {
        self.amount_bits == other.amount_bits
            && value_ptr_eq(&self.foreground, &other.foreground)
            && value_ptr_eq(&self.background, &other.background)
    }
}

struct CompletedMixJob {
    fingerprint: MixFingerprint,
    pixels: Vec<f32>,
    width: u32,
    height: u32,
}

/// Crossfades two pixel sources by a single uniform AMOUNT - not a
/// revival of the "MIX vs generic MASK" decision (see
/// ANIMATION_IMPLEMENTATION_PLAN.md's MIX section): MASK modulates one
/// operation's own effect strength via another node's per-pixel alpha,
/// this blends two independent sources by one scalar. Exists as a
/// purpose-built, always-eligible target for animation wiring - unlike
/// most operations, MIX's AMOUNT is a Number parameter every instance
/// has, regardless of what the two blended sources are.
pub struct Mix {
    pub amount: f64,
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale.
    gpu_pipeline: RefCell<Option<MixGpuPipeline>>,
    pending: Rc<RefCell<Option<MixFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedMixJob>>>,
}

impl Mix {
    pub fn new() -> Self {
        Self {
            amount: 0.5,
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Add::dispatch_gpu` in structure, with the AMOUNT param
    /// packed the same way BLUR's MIN/MAX are.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: MixFingerprint, foreground: FloatImage, background: FloatImage) {
        let width = foreground.width;
        let height = foreground.height;
        let len = foreground.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_mix_pipeline(&gpu));
            }
        }

        let foreground_buffer = gpu.upload("mix foreground", &foreground.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let background_buffer = gpu.upload("mix background", &background.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "mix output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let amount_bits = (f64::from_bits(fingerprint.amount_bits) as f32).to_bits();
        let params: [u32; 4] = [width, height, amount_bits, 0];
        let params_buffer = gpu.upload("mix params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "mix readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("mix bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: foreground_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: background_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("mix dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pixels = gpu.read_buffer_blocking(&readback_buffer, len);
            *self.last_gpu_result.borrow_mut() = Some(CompletedMixJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pixels = gpu.read_buffer_async(&readback_buffer, len).await;
                *last_gpu_result.borrow_mut() = Some(CompletedMixJob {
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

    /// Per-channel crossfade, all 4 channels uniformly (same convention
    /// as Add/Multiply/Screen) - NOT Ghost's alpha-aware Porter-Duff
    /// "over"; this is a plain lerp, not a compositing operator.
    pub fn mix_pixels(a: &[f32], b: &[f32], amount: f64) -> Vec<f32> {
        let amount = amount as f32;
        let mut output = vec![0f32; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            for channel in 0..4 {
                target[channel] = source_a[channel] * (1.0 - amount) + source_b[channel] * amount;
            }
        }

        output
    }
}

impl Default for Mix {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Mix {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "mix",
            menu: "COMPOSE",
            label: "MIX",
            action: None,
            ui_action: None,
            create_node: Some("mix"),
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
            display_name: "Mix",
            category: OperationCategory::Composite,
            inputs: vec![
                InputDescriptor { kind: Input::Foreground, accepts: PIXEL_KINDS },
                InputDescriptor { kind: Input::Background, accepts: PIXEL_KINDS },
            ],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "AMOUNT",
                kind: ParameterKind::Number { step: 0.01, min: Some(0.0), max: Some(1.0) },
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "AMOUNT" => Some(Value::Number(self.amount)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("AMOUNT", Value::Number(v)) => {
                if !(0.0..=1.0).contains(&v) {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.amount = v;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    // See blur.rs's identical override for the full rationale.
    fn is_live(&self) -> bool {
        self.pending.borrow().is_some()
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(first) = find_input(inputs, Input::Foreground) else {
            return Err(OperationError::InvalidInputType("Mix requires FOREGROUND".into()));
        };

        let Some(second) = find_input(inputs, Input::Background) else {
            return Err(OperationError::InvalidInputType("Mix requires BACKGROUND".into()));
        };

        let first_image = FloatImage::from_value(first, ctx)?;
        let second_image = FloatImage::from_value(second, ctx)?;

        if first_image.width != second_image.width || first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Mix inputs must have matching dimensions".into()
            ));
        }

        // No MASK input exists for this operation - GPU dispatch applies
        // unconditionally when available.
        if let Some(gpu) = ctx.gpu.clone() {
            let fingerprint = MixFingerprint { foreground: first.clone(), background: second.clone(), amount_bits: self.amount.to_bits() };

            let cached = self.last_gpu_result.borrow().as_ref()
                .filter(|completed| completed.fingerprint.matches(&fingerprint))
                .map(|completed| FloatImage { pixels: completed.pixels.clone(), width: completed.width, height: completed.height });

            if let Some(result) = cached {
                return Ok(vec![Value::FloatImage(Arc::new(result))]);
            }

            let already_pending = self.pending.borrow().as_ref().is_some_and(|p| p.matches(&fingerprint));
            if !already_pending {
                self.dispatch_gpu(gpu, fingerprint, first_image.clone(), second_image.clone());
            }
        }

        let mixed = Self::mix_pixels(&first_image.pixels, &second_image.pixels, self.amount);

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: mixed,
            width: first_image.width,
            height: first_image.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Mix::new())
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

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<crate::graphics::U8Image> {
        Arc::new(crate::graphics::U8Image { pixels, width, height, format: crate::graphics::ImageFormat::Rgba8 })
    }

    #[test]
    fn amount_zero_is_pure_foreground() {
        let a = FloatImage::from_image(&image(vec![10, 20, 30, 255], 1, 1)).pixels;
        let b = FloatImage::from_image(&image(vec![200, 210, 220, 255], 1, 1)).pixels;
        let out = Mix::mix_pixels(&a, &b, 0.0);
        assert_eq!(out, a);
    }

    #[test]
    fn amount_one_is_pure_background() {
        let a = FloatImage::from_image(&image(vec![10, 20, 30, 255], 1, 1)).pixels;
        let b = FloatImage::from_image(&image(vec![200, 210, 220, 255], 1, 1)).pixels;
        let out = Mix::mix_pixels(&a, &b, 1.0);
        assert_eq!(out, b);
    }

    #[test]
    fn amount_half_averages_both_inputs() {
        let a = vec![0.0, 0.0, 0.0, 0.0];
        let b = vec![1.0, 1.0, 1.0, 1.0];
        let out = Mix::mix_pixels(&a, &b, 0.5);
        for c in out {
            assert!((c - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn set_parameter_rejects_an_amount_above_one() {
        let mut mix = Mix::new();
        let err = mix.set_parameter("AMOUNT", Value::Number(1.5)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn set_parameter_rejects_a_negative_amount() {
        let mut mix = Mix::new();
        let err = mix.set_parameter("AMOUNT", Value::Number(-0.1)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn execute_errors_without_a_wired_foreground() {
        let mix = Mix::new();
        let bg = Value::Image(image(vec![0, 0, 0, 255], 1, 1));
        let err = mix.execute(&context(1, 1), &[(Input::Background, bg)]).unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn execute_errors_without_a_wired_background() {
        let mix = Mix::new();
        let fg = Value::Image(image(vec![0, 0, 0, 255], 1, 1));
        let err = mix.execute(&context(1, 1), &[(Input::Foreground, fg)]).unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn execute_errors_on_mismatched_dimensions() {
        let mix = Mix::new();
        let fg = Value::Image(image(vec![0, 0, 0, 255, 0, 0, 0, 255], 2, 1));
        let bg = Value::Image(image(vec![0, 0, 0, 255], 1, 1));
        let err = mix
            .execute(&context(2, 1), &[(Input::Foreground, fg), (Input::Background, bg)])
            .unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn mix_combines_two_wired_inputs_by_amount() {
        let mut mix = Mix::new();
        mix.amount = 0.25;

        let fg = Value::Image(image(vec![0, 0, 0, 0], 1, 1));
        let bg = Value::Image(image(vec![255, 255, 255, 255], 1, 1));

        let values = mix
            .execute(&context(1, 1), &[(Input::Foreground, fg), (Input::Background, bg)])
            .unwrap();

        match &values[0] {
            Value::FloatImage(out) => {
                assert!((out.pixels[0] - 0.25).abs() < 1e-6);
                assert!((out.pixels[3] - 0.25).abs() < 1e-6);
            }
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    // --- WebGPU Phase 1.3 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let mix = Mix::new();
        assert!(!mix.is_live());

        *mix.pending.borrow_mut() = Some(MixFingerprint { foreground: Value::Number(0.0), background: Value::Number(0.0), amount_bits: 0.5f64.to_bits() });
        assert!(mix.is_live());

        *mix.pending.borrow_mut() = None;
        assert!(!mix.is_live());
    }

    #[test]
    fn gpu_mix_matches_cpu_within_tolerance_once_warmed_up() {
        let Ok(gpu) = pollster::block_on(crate::gpu::GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };
        let gpu = Arc::new(gpu);

        let width = 5;
        let height = 3;
        let fg = image((0..(width * height)).flat_map(|n| {
            let v = ((n * 53) % 256) as u8;
            [v, v.wrapping_add(20), v.wrapping_add(40), 255]
        }).collect(), width, height);
        let bg = image((0..(width * height)).flat_map(|n| {
            let v = ((n * 89) % 256) as u8;
            [v, v.wrapping_add(60), v.wrapping_add(120), 255]
        }).collect(), width, height);

        let mut cpu_mix = Mix::new();
        cpu_mix.amount = 0.35;
        let cpu_values = cpu_mix
            .execute(&context(width, height), &[(Input::Foreground, Value::Image(fg.clone())), (Input::Background, Value::Image(bg.clone()))])
            .unwrap();
        let Value::FloatImage(cpu_result) = &cpu_values[0] else { panic!("expected a float image") };

        let mut gpu_mix = Mix::new();
        gpu_mix.amount = 0.35;
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };
        let _ = gpu_mix.execute(&gpu_ctx, &[(Input::Foreground, Value::Image(fg.clone())), (Input::Background, Value::Image(bg.clone()))]).unwrap();
        let gpu_values = gpu_mix.execute(&gpu_ctx, &[(Input::Foreground, Value::Image(fg)), (Input::Background, Value::Image(bg))]).unwrap();
        let Value::FloatImage(gpu_result) = &gpu_values[0] else { panic!("expected a float image") };

        assert_eq!(cpu_result.pixels.len(), gpu_result.pixels.len());
        for (index, (cpu_px, gpu_px)) in cpu_result.pixels.iter().zip(gpu_result.pixels.iter()).enumerate() {
            assert!((cpu_px - gpu_px).abs() < 1e-4, "channel {}: cpu={}, gpu={}", index, cpu_px, gpu_px);
        }
    }
}
