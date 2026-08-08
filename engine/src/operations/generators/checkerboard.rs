// src/operations/generator/checkerboard.rs

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::compositor::{
    Context,
    Operation,
    OperationDescriptor,
    OperationError,
    Input,
    Value,
    metadata::{
        OperationCategory,
        OperationMetadata,
        OutputKind,
        ParameterDescriptor,
        ParameterKind,
    },
};

use crate::graphics::{
    Color,
    U8Image,
    ImageFormat,
};
use crate::gpu::GpuState;

// GPU compute shader - SPECwebgpuoperations.md's Phase 1.2. CHECKERBOARD
// has no wired inputs at all (not even SOURCE) - "zero buffer" per the
// spec's own phase title - so the bind group is simpler than Phase 1.1's:
// only an output storage buffer and a uniform params buffer, no input
// storage buffer. Correspondingly, the fingerprint below has no `Value`
// to compare via `value_ptr_eq` either - the output is purely a function
// of (width, height, size, color_a, color_b), all read directly from
// `ctx.meta`/`self`, so the fingerprint captures those directly instead.
const CHECKERBOARD_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read_write> output: array<f32>;
    @group(0) @binding(1) var<uniform> params: array<vec4<u32>, 3>;

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params[0].x;
        let height = params[0].y;
        let tile = params[0].z;

        if (id.x >= width || id.y >= height) {
            return;
        }

        let color_a = vec4<f32>(
            bitcast<f32>(params[0].w),
            bitcast<f32>(params[1].x),
            bitcast<f32>(params[1].y),
            bitcast<f32>(params[1].z),
        );
        let color_b = vec4<f32>(
            bitcast<f32>(params[1].w),
            bitcast<f32>(params[2].x),
            bitcast<f32>(params[2].y),
            bitcast<f32>(params[2].z),
        );

        let checker = ((id.x / tile) + (id.y / tile)) % 2u == 0u;
        let color = select(color_b, color_a, checker);

        let idx = (id.y * width + id.x) * 4u;
        output[idx] = color.x;
        output[idx + 1u] = color.y;
        output[idx + 2u] = color.z;
        output[idx + 3u] = color.w;
    }
"#;

struct CheckerboardGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_checkerboard_pipeline(gpu: &GpuState) -> CheckerboardGpuPipeline {
    let shader = gpu.create_shader(CHECKERBOARD_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("checkerboard bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
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

    let pipeline = gpu.create_compute_pipeline("checkerboard pipeline", &shader, "main", &[&bind_group_layout]);

    CheckerboardGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct CheckerboardFingerprint {
    width: u32,
    height: u32,
    tile: u32,
    color_a: Color,
    color_b: Color,
}

impl CheckerboardFingerprint {
    fn matches(&self, other: &CheckerboardFingerprint) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.tile == other.tile
            && self.color_a == other.color_a
            && self.color_b == other.color_b
    }
}

struct CompletedCheckerboardJob {
    fingerprint: CheckerboardFingerprint,
    // Already quantized to u8 (see dispatch_gpu's own comment - matches
    // to_rgba_u8's truncating cast exactly, not to_image_clamped's
    // rounding one, since this operation calls to_rgba_u8 on its colors).
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

pub struct Checkerboard {
    pub size: f64,
    pub color_a: Color,
    pub color_b: Color,
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale.
    gpu_pipeline: RefCell<Option<CheckerboardGpuPipeline>>,
    pending: Rc<RefCell<Option<CheckerboardFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedCheckerboardJob>>>,
}

impl Checkerboard {
    pub fn new() -> Self {
        Self {
            size: 32.0,
            color_a: Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            color_b: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Blur::dispatch_gpu` in structure, with two differences:
    /// no input buffer (nothing to upload - the shader is purely
    /// procedural), and the read-back `Vec<f32>` is quantized to
    /// `Vec<u8>` via the same truncating cast `Color::to_rgba_u8` uses
    /// (`(c.clamp(0.0, 1.0) * 255.0) as u8` - deliberately *not*
    /// `.round()`, unlike `Clamp`'s `to_image_clamped`), since this
    /// operation's own colors go through `to_rgba_u8` on the CPU path.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: CheckerboardFingerprint) {
        let width = fingerprint.width;
        let height = fingerprint.height;
        let len = (width as usize) * (height as usize) * 4;
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_checkerboard_pipeline(&gpu));
            }
        }

        let output_buffer = gpu.create_buffer(
            "checkerboard output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let color_a_bits = [
            fingerprint.color_a.r.to_bits(), fingerprint.color_a.g.to_bits(),
            fingerprint.color_a.b.to_bits(), fingerprint.color_a.a.to_bits(),
        ];
        let color_b_bits = [
            fingerprint.color_b.r.to_bits(), fingerprint.color_b.g.to_bits(),
            fingerprint.color_b.b.to_bits(), fingerprint.color_b.a.to_bits(),
        ];
        let params: [u32; 12] = [
            width, height, fingerprint.tile, color_a_bits[0],
            color_a_bits[1], color_a_bits[2], color_a_bits[3], color_b_bits[0],
            color_b_bits[1], color_b_bits[2], color_b_bits[3], 0,
        ];
        let params_buffer = gpu.upload("checkerboard params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "checkerboard readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("checkerboard bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("checkerboard dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let raw = gpu.read_buffer_blocking(&readback_buffer, len);
            let pixels: Vec<u8> = raw.iter().map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8).collect();
            *self.last_gpu_result.borrow_mut() = Some(CompletedCheckerboardJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let raw = gpu.read_buffer_async(&readback_buffer, len).await;
                let pixels: Vec<u8> = raw.iter().map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8).collect();
                *last_gpu_result.borrow_mut() = Some(CompletedCheckerboardJob {
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

    pub fn generate(&self, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        let a = self.color_a.to_rgba_u8();
        let b = self.color_b.to_rgba_u8();

        let tile = self.size.max(1.0) as u32;

        for y in 0..height {
            for x in 0..width {
                let checker = ((x / tile) + (y / tile)) % 2 == 0;

                let color = if checker { a } else { b };

                let index = ((y * width + x) * 4) as usize;

                pixels[index..index + 4].copy_from_slice(&color);
            }
        }

        pixels
    }
}

impl Operation for Checkerboard {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "checkerboard",
            menu: "GENERATE",
            label: "CHECKERBOARD",
            action: None,
            ui_action: None,
            create_node: Some("checkerboard"),
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
            display_name: "Checkerboard",
            category: OperationCategory::Generator,
            inputs: vec![],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "size",
                kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "A",
                kind: ParameterKind::Color,
                group: Some("COLOUR"),
            },
            ParameterDescriptor {
                name: "B",
                kind: ParameterKind::Color,
                group: Some("COLOUR"),
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "size" => Some(Value::Number(self.size)),
            "A" => Some(Value::Color(self.color_a)),
            "B" => Some(Value::Color(self.color_b)),
            _ => None,
        }
    }

    fn set_parameter(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<(), OperationError> {
        match name {
            "size" => {
                if let Value::Number(v) = value {
                    self.size = v.max(1.0);
                    Ok(())
                } else {
                    Err(OperationError::InvalidParameterType(name.to_string()))
                }
            }

            "A" => {
                if let Value::Color(color) = value {
                    self.color_a = color;
                    Ok(())
                } else {
                    Err(OperationError::InvalidParameterType(name.to_string()))
                }
            }

            "B" => {
                if let Value::Color(color) = value {
                    self.color_b = color;
                    Ok(())
                } else {
                    Err(OperationError::InvalidParameterType(name.to_string()))
                }
            }

            _ => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    // See blur.rs's identical override for the full rationale.
    fn is_live(&self) -> bool {
        self.pending.borrow().is_some()
    }

    fn execute(
        &self,
        ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        // No wired inputs at all - GPU dispatch applies unconditionally
        // when available, keyed on (width, height, size, color_a,
        // color_b) directly rather than a wired Value's pointer identity.
        if let Some(gpu) = ctx.gpu.clone() {
            let tile = self.size.max(1.0) as u32;
            let fingerprint = CheckerboardFingerprint {
                width: ctx.meta.width,
                height: ctx.meta.height,
                tile,
                color_a: self.color_a,
                color_b: self.color_b,
            };

            let cached = self.last_gpu_result.borrow().as_ref()
                .filter(|completed| completed.fingerprint.matches(&fingerprint))
                .map(|completed| U8Image { pixels: completed.pixels.clone(), width: completed.width, height: completed.height, format: ImageFormat::Rgba8 });

            if let Some(result) = cached {
                return Ok(vec![Value::Image(Arc::new(result))]);
            }

            let already_pending = self.pending.borrow().as_ref().is_some_and(|p| p.matches(&fingerprint));
            if !already_pending {
                self.dispatch_gpu(gpu, fingerprint);
            }
        }

        Ok(vec![
            Value::Image(Arc::new(U8Image {
                pixels: self.generate(
                    ctx.meta.width,
                    ctx.meta.height,
                ),
                width: ctx.meta.width,
                height: ctx.meta.height,
                format: ImageFormat::Rgba8,
            }))
        ])
    }
}


// Inventory registration for Checkerboard
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Checkerboard::new())
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

    #[test]
    fn generates_a_checkerboard_pattern() {
        let checkerboard = Checkerboard::new();

        let values = checkerboard
            .execute(&context(4, 4), &[])
            .expect("checkerboard should generate");

        match &values[0] {
            Value::Image(image) => {
                assert_eq!(image.width, 4);
                assert_eq!(image.height, 4);

                // default size is 32, so entire image is first color
                assert_eq!(
                    image.pixels[0..4],
                    [255, 255, 255, 255]
                );
            }

            other => panic!("expected image, got {:?}", other),
        }
    }

    #[test]
    fn changes_tile_size() {
        let mut checkerboard = Checkerboard::new();
        checkerboard.size = 1.0;

        let values = checkerboard
            .execute(&context(2, 2), &[])
            .expect("checkerboard should generate");

        match &values[0] {
            Value::Image(image) => {
                assert_eq!(
                    image.pixels,
                    vec![
                        255,255,255,255,
                        0,0,0,255,
                        0,0,0,255,
                        255,255,255,255,
                    ]
                );
            }

            other => panic!("expected image, got {:?}", other),
        }
    }

    // --- WebGPU Phase 1.2 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let checkerboard = Checkerboard::new();
        assert!(!checkerboard.is_live());

        *checkerboard.pending.borrow_mut() = Some(CheckerboardFingerprint {
            width: 4,
            height: 4,
            tile: 1,
            color_a: Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
            color_b: Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
        });
        assert!(checkerboard.is_live());

        *checkerboard.pending.borrow_mut() = None;
        assert!(!checkerboard.is_live());
    }

    #[test]
    fn gpu_checkerboard_matches_cpu_within_tolerance_once_warmed_up() {
        let Ok(gpu) = pollster::block_on(crate::gpu::GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };
        let gpu = Arc::new(gpu);

        let width = 6;
        let height = 5;

        let mut cpu_checkerboard = Checkerboard::new();
        cpu_checkerboard.size = 2.0;
        cpu_checkerboard.color_a = Color { r: 0.8, g: 0.2, b: 0.1, a: 1.0 };
        cpu_checkerboard.color_b = Color { r: 0.1, g: 0.3, b: 0.9, a: 0.5 };
        let cpu_values = cpu_checkerboard.execute(&context(width, height), &[]).unwrap();
        let Value::Image(cpu_result) = &cpu_values[0] else { panic!("expected an image") };

        let mut gpu_checkerboard = Checkerboard::new();
        gpu_checkerboard.size = 2.0;
        gpu_checkerboard.color_a = Color { r: 0.8, g: 0.2, b: 0.1, a: 1.0 };
        gpu_checkerboard.color_b = Color { r: 0.1, g: 0.3, b: 0.9, a: 0.5 };
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };
        let _ = gpu_checkerboard.execute(&gpu_ctx, &[]).unwrap();
        let gpu_values = gpu_checkerboard.execute(&gpu_ctx, &[]).unwrap();
        let Value::Image(gpu_result) = &gpu_values[0] else { panic!("expected an image") };

        assert_eq!(cpu_result.pixels, gpu_result.pixels);
    }
}