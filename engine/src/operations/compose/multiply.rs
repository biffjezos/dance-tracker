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
    metadata::{ InputDescriptor, OperationCategory, OperationMetadata, OutputKind, PIXEL_KINDS },
    value::value_ptr_eq,
    Value,
};
use crate::graphics::FloatImage;
use crate::gpu::GpuState;

// GPU compute shader - SPECwebgpuoperations.md's Phase 1.3. Same
// bind-group shape as ADD's. MULTIPLY, unlike ADD/SCREEN/SUBTRACT,
// hasn't been migrated to bbox-consumption yet - its masked path is
// still an unrestricted full-frame compute + apply_mask, not a
// work_area-restricted one. GPU dispatch still applies only when
// `mask.is_none()`, same blanket-rule split as every other operation -
// the masked path is left entirely untouched (still CPU, still
// unrestricted), rather than assuming it's safe to accelerate just
// because it happens to already be full-frame.
const MULTIPLY_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> foreground: array<f32>;
    @group(0) @binding(1) var<storage, read> background: array<f32>;
    @group(0) @binding(2) var<storage, read_write> output: array<f32>;
    @group(0) @binding(3) var<uniform> params: vec4<u32>;

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params.x;
        let height = params.y;

        if (id.x >= width || id.y >= height) {
            return;
        }

        let idx = (id.y * width + id.x) * 4u;
        output[idx] = foreground[idx] * background[idx];
        output[idx + 1u] = foreground[idx + 1u] * background[idx + 1u];
        output[idx + 2u] = foreground[idx + 2u] * background[idx + 2u];
        output[idx + 3u] = foreground[idx + 3u] * background[idx + 3u];
    }
"#;

struct MultiplyGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_multiply_pipeline(gpu: &GpuState) -> MultiplyGpuPipeline {
    let shader = gpu.create_shader(MULTIPLY_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("multiply bind group layout"),
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

    let pipeline = gpu.create_compute_pipeline("multiply pipeline", &shader, "main", &[&bind_group_layout]);

    MultiplyGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct MultiplyFingerprint {
    foreground: Value,
    background: Value,
}

impl MultiplyFingerprint {
    fn matches(&self, other: &MultiplyFingerprint) -> bool {
        value_ptr_eq(&self.foreground, &other.foreground) && value_ptr_eq(&self.background, &other.background)
    }
}

struct CompletedMultiplyJob {
    fingerprint: MultiplyFingerprint,
    pixels: Vec<f32>,
    width: u32,
    height: u32,
}

/// Multiply operation - multiplies RGBA channels from two inputs pixel by
/// pixel, unclamped: given two in-gamut (0.0..1.0) inputs the result can
/// never exceed either one, but multiplying an already out-of-gamut value
/// (e.g. 1.5 from an ADD result) correctly stays out of gamut (1.5 * 1.5 =
/// 2.25), not silently reclamped mid-calculation. Both inputs accept a
/// bounded Image or an already-unbounded FloatImage alike (via
/// FloatImage::from_value), so chaining another compose op's output
/// straight into Multiply works without an intervening CLAMP.
pub struct Multiply {
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale.
    gpu_pipeline: RefCell<Option<MultiplyGpuPipeline>>,
    pending: Rc<RefCell<Option<MultiplyFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedMultiplyJob>>>,
}

impl Multiply {
    pub fn new() -> Self {
        Self {
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Add::dispatch_gpu` in structure.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: MultiplyFingerprint, foreground: FloatImage, background: FloatImage) {
        let width = foreground.width;
        let height = foreground.height;
        let len = foreground.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_multiply_pipeline(&gpu));
            }
        }

        let foreground_buffer = gpu.upload("multiply foreground", &foreground.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let background_buffer = gpu.upload("multiply background", &background.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "multiply output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let params: [u32; 4] = [width, height, 0, 0];
        let params_buffer = gpu.upload("multiply params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "multiply readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("multiply bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: foreground_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: background_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("multiply dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pixels = gpu.read_buffer_blocking(&readback_buffer, len);
            *self.last_gpu_result.borrow_mut() = Some(CompletedMultiplyJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pixels = gpu.read_buffer_async(&readback_buffer, len).await;
                *last_gpu_result.borrow_mut() = Some(CompletedMultiplyJob {
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

    /// Multiply two RGBA pixel buffers channel by channel - NOT clamped.
    /// See this module's own doc comment for why.
    pub fn multiply_pixels(a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut output = vec![0f32; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            target[0] = source_a[0] * source_b[0];
            target[1] = source_a[1] * source_b[1];
            target[2] = source_a[2] * source_b[2];
            target[3] = source_a[3] * source_b[3];
        }

        output
    }
}

impl Operation for Multiply {

    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "multiply",
            menu: "COMPOSE",
            label: "MULTIPLY",
            action: None,
            ui_action: None,
            create_node: Some("multiply"),
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
            display_name: "Multiply",
            category: OperationCategory::Color,
            // Identity (MASK=0) is Foreground unmodified - see add.rs's
            // metadata() for why Foreground and not Background.
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

    fn set_parameter(
        &mut self,
        name: &str,
        _value: Value,
    ) -> Result<(), OperationError> {
        Err(OperationError::UnknownParameter(name.to_string()))
    }

    // See blur.rs's identical override for the full rationale.
    fn is_live(&self) -> bool {
        self.pending.borrow().is_some()
    }

    fn execute(
        &self,
        ctx: &Context,
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {

        let Some(first) = find_input(inputs, Input::Foreground) else {
            return Err(OperationError::InvalidInputType(
                "Multiply requires first input".into()
            ));
        };

        let Some(second) = find_input(inputs, Input::Background) else {
            return Err(OperationError::InvalidInputType(
                "Multiply requires second input".into()
            ));
        };

        let first_image = FloatImage::from_value(first, ctx)?;
        let second_image = FloatImage::from_value(second, ctx)?;

        if first_image.width != second_image.width ||
           first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Multiply inputs must have matching dimensions".into()
            ));
        }

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        // MULTIPLY isn't migrated to bbox-consumption (see this module's
        // own GPU shader doc comment) - the masked path stays exactly as
        // it was, full-frame CPU compute + apply_mask, completely
        // untouched by GPU dispatch.
        if let Some(mask) = &mask {
            let multiplied = Self::multiply_pixels(&first_image.pixels, &second_image.pixels);
            let multiplied = crate::graphics::apply_mask(
                &first_image.pixels,
                multiplied,
                Some(mask),
                first_image.width,
                first_image.height,
            )?;

            return Ok(vec![
                Value::FloatImage(Arc::new(FloatImage {
                    pixels: multiplied,
                    width: first_image.width,
                    height: first_image.height,
                }))
            ]);
        }

        // Unmasked path: try GPU first when available.
        if let Some(gpu) = ctx.gpu.clone() {
            let fingerprint = MultiplyFingerprint { foreground: first.clone(), background: second.clone() };

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

        let multiplied = Self::multiply_pixels(&first_image.pixels, &second_image.pixels);

        Ok(vec![
            Value::FloatImage(Arc::new(FloatImage {
                pixels: multiplied,
                width: first_image.width,
                height: first_image.height,
            }))
        ])
    }
}

// Inventory registration for Multiply
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Multiply::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<crate::graphics::U8Image> {
        Arc::new(crate::graphics::U8Image {
            pixels,
            width,
            height,
            format: crate::graphics::ImageFormat::Rgba8,
        })
    }

    fn float_pixels(pixels: Vec<u8>) -> Vec<f32> {
        FloatImage::from_image(&image(pixels, 1, 1)).pixels
    }

    fn as_u8_pixels(value: &Value) -> Vec<u8> {
        match value {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels,
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn multiplying_by_white_is_identity() {
        let white = float_pixels(vec![255, 255, 255, 255]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Multiply::multiply_pixels(&white, &FloatImage::from_image(&color).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);

        assert_eq!(out.pixels, color.pixels);
    }

    #[test]
    fn multiplying_by_black_is_black() {
        let black = float_pixels(vec![0, 0, 0, 255]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Multiply::multiply_pixels(&black, &FloatImage::from_image(&color).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);

        assert_eq!(out.pixels, vec![0, 0, 0, 200]);
    }

    #[test]
    fn multiplying_two_out_of_gamut_values_stays_out_of_gamut() {
        // Regression: MULTIPLY used to only accept a bounded
        // Image/Frame/Video and clamp inline via u16/255 math - both
        // of which broke once a compose op's own output (FloatImage)
        // could be wired straight into it.
        let a = vec![1.5f32, 1.5, 1.5, 1.0];
        let b = vec![1.5f32, 1.5, 1.5, 1.0];

        let out = Multiply::multiply_pixels(&a, &b);

        assert!((out[0] - 2.25).abs() < 0.001);
    }

    #[test]
    fn multiply_in_graph_requires_both_inputs_of_matching_size() {
        let multiply = Multiply::new();

        let a = Value::Image(image(vec![255, 0, 0, 255], 1, 1));
        let b = Value::Image(image(vec![0, 0, 0, 255, 0, 0, 0, 255], 2, 1));

        let err = multiply
            .execute(&context(1, 1), &[(Input::Foreground, a), (Input::Background, b)])
            .unwrap_err();

        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn multiply_combines_two_wired_inputs() {
        let multiply = Multiply::new();

        let fg = Value::Image(image(vec![255, 255, 255, 255], 1, 1));
        let bg = Value::Image(image(vec![10, 20, 30, 255], 1, 1));

        let values = multiply
            .execute(&context(1, 1), &[(Input::Foreground, fg), (Input::Background, bg)])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), vec![10, 20, 30, 255]);
    }

    #[test]
    fn a_zero_alpha_mask_passes_through_foreground_unmultiplied() {
        let multiply = Multiply::new();
        let fg = image(vec![10, 20, 30, 255], 1, 1);
        let bg = image(vec![0, 0, 0, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 0], 1, 1);

        let values = multiply
            .execute(&context(1, 1), &[
                (Input::Foreground, Value::Image(fg.clone())),
                (Input::Background, Value::Image(bg)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), fg.pixels);
    }

    #[test]
    fn a_mismatched_mask_size_errors_instead_of_being_silently_ignored() {
        let multiply = Multiply::new();
        let fg = image(vec![10, 20, 30, 255], 1, 1);
        let bg = image(vec![0, 0, 0, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 255, 0, 0, 0, 255], 2, 1);

        let err = multiply
            .execute(&context(1, 1), &[
                (Input::Foreground, Value::Image(fg)),
                (Input::Background, Value::Image(bg)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap_err();

        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    // --- WebGPU Phase 1.3 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let multiply = Multiply::new();
        assert!(!multiply.is_live());

        *multiply.pending.borrow_mut() = Some(MultiplyFingerprint { foreground: Value::Number(0.0), background: Value::Number(0.0) });
        assert!(multiply.is_live());

        *multiply.pending.borrow_mut() = None;
        assert!(!multiply.is_live());
    }

    #[test]
    fn gpu_multiply_matches_cpu_within_tolerance_once_warmed_up() {
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

        let cpu_multiply = Multiply::new();
        let cpu_values = cpu_multiply
            .execute(&context(width, height), &[(Input::Foreground, Value::Image(fg.clone())), (Input::Background, Value::Image(bg.clone()))])
            .unwrap();
        let Value::FloatImage(cpu_result) = &cpu_values[0] else { panic!("expected a float image") };

        let gpu_multiply = Multiply::new();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };
        let _ = gpu_multiply.execute(&gpu_ctx, &[(Input::Foreground, Value::Image(fg.clone())), (Input::Background, Value::Image(bg.clone()))]).unwrap();
        let gpu_values = gpu_multiply.execute(&gpu_ctx, &[(Input::Foreground, Value::Image(fg)), (Input::Background, Value::Image(bg))]).unwrap();
        let Value::FloatImage(gpu_result) = &gpu_values[0] else { panic!("expected a float image") };

        assert_eq!(cpu_result.pixels.len(), gpu_result.pixels.len());
        for (index, (cpu_px, gpu_px)) in cpu_result.pixels.iter().zip(gpu_result.pixels.iter()).enumerate() {
            assert!((cpu_px - gpu_px).abs() < 1e-4, "channel {}: cpu={}, gpu={}", index, cpu_px, gpu_px);
        }
    }
}
