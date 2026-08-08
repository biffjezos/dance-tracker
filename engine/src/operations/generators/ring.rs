// src/operations/generators/ring.rs
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

use crate::graphics::{Color, U8Image, ImageFormat};
use crate::gpu::GpuState;

// GPU compute shader - SPECwebgpuoperations.md's Phase 1.2. RING's
// per-ring `colors: Vec<Color>` has no fixed upper bound (COUNT's own
// `max: None`), so it can't fit a fixed-size uniform buffer the way
// CHECKERBOARD's two colors do. Uses a runtime-sized storage buffer
// instead (`array<vec4<f32>>`, length read via `arrayLength()`) - the
// exact same WGSL mechanism `gpu/mod.rs`'s own DOUBLE_SHADER test
// already relies on (`id.x < arrayLength(&input)`), just applied to a
// per-ring color table instead of pixel data. This is still "zero
// buffer" per the phase's own framing in the sense that matters (no
// wired SOURCE/input pixel buffer) - the colors buffer holds this
// operation's own parameter data, not an input.
const RING_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read_write> output: array<f32>;
    @group(0) @binding(1) var<storage, read> colors: array<vec4<f32>>;
    @group(0) @binding(2) var<uniform> params: array<vec4<u32>, 2>;

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params[0].x;
        let height = params[0].y;
        let count = params[0].z;
        let radius = bitcast<f32>(params[0].w);
        let spacing = bitcast<f32>(params[1].x);
        let thickness = bitcast<f32>(params[1].y);

        if (id.x >= width || id.y >= height) {
            return;
        }

        let cx = f32(width) / 2.0;
        let cy = f32(height) / 2.0;
        let dx = f32(id.x) + 0.5 - cx;
        let dy = f32(id.y) + 0.5 - cy;
        let dist = sqrt(dx * dx + dy * dy);
        let half_thickness = max(thickness, 0.0) / 2.0;

        var out_color = vec4<f32>(0.0, 0.0, 0.0, 0.0);

        for (var ring_number = 1u; ring_number <= count; ring_number = ring_number + 1u) {
            let ring_radius = radius - (f32(ring_number) - 1.0) * spacing;
            if (ring_radius < 0.0) {
                continue;
            }
            if (abs(dist - ring_radius) <= half_thickness) {
                out_color = colors[ring_number - 1u];
                break;
            }
        }

        let idx = (id.y * width + id.x) * 4u;
        output[idx] = out_color.x;
        output[idx + 1u] = out_color.y;
        output[idx + 2u] = out_color.z;
        output[idx + 3u] = out_color.w;
    }
"#;

struct RingGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_ring_pipeline(gpu: &GpuState) -> RingGpuPipeline {
    let shader = gpu.create_shader(RING_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ring bind group layout"),
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
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline = gpu.create_compute_pipeline("ring pipeline", &shader, "main", &[&bind_group_layout]);

    RingGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct RingFingerprint {
    width: u32,
    height: u32,
    count: usize,
    radius_bits: u64,
    spacing_bits: u64,
    thickness_bits: u64,
    colors: Vec<Color>,
}

impl RingFingerprint {
    fn matches(&self, other: &RingFingerprint) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.count == other.count
            && self.radius_bits == other.radius_bits
            && self.spacing_bits == other.spacing_bits
            && self.thickness_bits == other.thickness_bits
            && self.colors == other.colors
    }
}

struct CompletedRingJob {
    fingerprint: RingFingerprint,
    // Already quantized to u8, matching to_rgba_u8's truncating cast
    // exactly (see CHECKERBOARD's identical comment).
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

/// Concentric rings, sized like Saturn's rings: RADIUS is the outer
/// edge of the whole set, SPACING is the gap between consecutive rings,
/// THICKNESS is the (uniform) stroke width of every ring. Static - no
/// time dependency, no `is_live()` - purely a function of its own
/// parameters, same as Checkerboard. Each ring gets its own colour via
/// RING_SELECTOR (bounded by the live COUNT) + RING_COLOR, in a
/// "COLOUR" parameter group - the same deep-menu mechanism
/// Checkerboard's A/B colours already use, just with an index selector
/// instead of two fixed named colours.
pub struct Ring {
    pub count: usize,
    pub radius: f64,
    pub spacing: f64,
    pub thickness: f64,
    selected_ring: usize, // 1-based, always in 1..=count
    colors: Vec<Color>,   // always exactly `count` long
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale.
    gpu_pipeline: RefCell<Option<RingGpuPipeline>>,
    pending: Rc<RefCell<Option<RingFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedRingJob>>>,
}

impl Ring {
    pub fn new() -> Self {
        Self {
            count: 1,
            radius: 64.0,
            spacing: 16.0,
            thickness: 4.0,
            selected_ring: 1,
            colors: vec![Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }],
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Blur::dispatch_gpu` in structure - see `CHECKERBOARD`'s
    /// identical doc comment for the no-input-buffer and truncating-
    /// quantization notes, both of which apply here too. The one
    /// RING-specific addition is the `colors` storage buffer, uploaded
    /// alongside the uniform scalar params.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: RingFingerprint) {
        let width = fingerprint.width;
        let height = fingerprint.height;
        let len = (width as usize) * (height as usize) * 4;
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_ring_pipeline(&gpu));
            }
        }

        let output_buffer = gpu.create_buffer(
            "ring output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );

        let color_floats: Vec<f32> = fingerprint.colors.iter().flat_map(|c| [c.r, c.g, c.b, c.a]).collect();
        let colors_buffer = gpu.upload("ring colors", &color_floats, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);

        let params: [u32; 8] = [
            width, height, fingerprint.count as u32,
            (f64::from_bits(fingerprint.radius_bits) as f32).to_bits(),
            (f64::from_bits(fingerprint.spacing_bits) as f32).to_bits(),
            (f64::from_bits(fingerprint.thickness_bits) as f32).to_bits(),
            0, 0,
        ];
        let params_buffer = gpu.upload("ring params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "ring readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("ring bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: colors_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("ring dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let raw = gpu.read_buffer_blocking(&readback_buffer, len);
            let pixels: Vec<u8> = raw.iter().map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8).collect();
            *self.last_gpu_result.borrow_mut() = Some(CompletedRingJob { fingerprint, pixels, width, height });
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
                *last_gpu_result.borrow_mut() = Some(CompletedRingJob {
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

    /// Resize `colors` to exactly `new_count` entries, cloning the last
    /// ring's colour into any newly-added slots (a reasonable default
    /// fill, not left unexplained - see ANIMATION_IMPLEMENTATION_PLAN.md's
    /// RING section) and clamping `selected_ring` back into range if it
    /// no longer fits.
    fn set_count(&mut self, new_count: usize) {
        let new_count = new_count.max(1);

        if new_count > self.colors.len() {
            let fill = *self.colors.last().unwrap();
            self.colors.resize(new_count, fill);
        } else {
            self.colors.truncate(new_count);
        }

        self.count = new_count;
        self.selected_ring = self.selected_ring.min(self.count);
    }

    /// Ring `n`'s (1-based) own radius - ring 1 is outermost, at
    /// `RADIUS`; each subsequent ring sits `SPACING` further in.
    fn ring_radius(&self, ring_number: usize) -> f64 {
        self.radius - (ring_number as f64 - 1.0) * self.spacing
    }

    pub fn generate(&self, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let half_thickness = self.thickness.max(0.0) / 2.0;

        for y in 0..height {
            for x in 0..width {
                let dx = x as f64 + 0.5 - cx;
                let dy = y as f64 + 0.5 - cy;
                let dist = (dx * dx + dy * dy).sqrt();

                let index = ((y * width + x) * 4) as usize;

                for ring_number in 1..=self.count {
                    let ring_radius = self.ring_radius(ring_number);
                    if ring_radius < 0.0 {
                        continue;
                    }
                    if (dist - ring_radius).abs() <= half_thickness {
                        let rgba = self.colors[ring_number - 1].to_rgba_u8();
                        pixels[index..index + 4].copy_from_slice(&rgba);
                        break;
                    }
                }
            }
        }

        pixels
    }
}

impl Default for Ring {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Ring {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "ring",
            menu: "GENERATE",
            label: "RING",
            action: None,
            ui_action: None,
            create_node: Some("ring"),
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
            display_name: "Ring",
            category: OperationCategory::Generator,
            inputs: vec![],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "COUNT",
                kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "RADIUS",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "SPACING",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "THICKNESS",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "RING_SELECTOR",
                kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: Some(self.count as f64) },
                group: Some("COLOUR"),
            },
            ParameterDescriptor {
                name: "RING_COLOR",
                kind: ParameterKind::Color,
                group: Some("COLOUR"),
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "COUNT" => Some(Value::Number(self.count as f64)),
            "RADIUS" => Some(Value::Number(self.radius)),
            "SPACING" => Some(Value::Number(self.spacing)),
            "THICKNESS" => Some(Value::Number(self.thickness)),
            "RING_SELECTOR" => Some(Value::Number(self.selected_ring as f64)),
            "RING_COLOR" => Some(Value::Color(self.colors[self.selected_ring - 1])),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("COUNT", Value::Number(v)) => {
                if v < 1.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.set_count(v.round() as usize);
                Ok(())
            }
            ("RADIUS", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.radius = v;
                Ok(())
            }
            ("SPACING", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.spacing = v;
                Ok(())
            }
            ("THICKNESS", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.thickness = v;
                Ok(())
            }
            ("RING_SELECTOR", Value::Number(v)) => {
                let index = v.round() as i64;
                if index < 1 || index as usize > self.count {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.selected_ring = index as usize;
                Ok(())
            }
            ("RING_COLOR", Value::Color(color)) => {
                let index = self.selected_ring - 1;
                self.colors[index] = color;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    // See blur.rs's identical override for the full rationale.
    fn is_live(&self) -> bool {
        self.pending.borrow().is_some()
    }

    fn execute(&self, ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        // No wired inputs at all - GPU dispatch applies unconditionally
        // when available, keyed on (width, height, count, radius,
        // spacing, thickness, colors) directly.
        if let Some(gpu) = ctx.gpu.clone() {
            let fingerprint = RingFingerprint {
                width: ctx.meta.width,
                height: ctx.meta.height,
                count: self.count,
                radius_bits: self.radius.to_bits(),
                spacing_bits: self.spacing.to_bits(),
                thickness_bits: self.thickness.to_bits(),
                colors: self.colors.clone(),
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
                pixels: self.generate(ctx.meta.width, ctx.meta.height),
                width: ctx.meta.width,
                height: ctx.meta.height,
                format: ImageFormat::Rgba8,
            }))
        ])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Ring::new())
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
    fn a_single_ring_draws_a_band_at_its_radius() {
        let mut ring = Ring::new();
        ring.radius = 2.0;
        ring.thickness = 1.0;

        let pixels = ring.generate(8, 8);
        // Centre pixel is far from radius 2 - must stay transparent.
        let centre_index = ((4 * 8 + 4) * 4) as usize;
        assert_eq!(&pixels[centre_index..centre_index + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn count_bounds_the_ring_selector_parameter() {
        let mut ring = Ring::new();
        ring.set_parameter("COUNT", Value::Number(3.0)).unwrap();

        let selector = ring.parameters().into_iter().find(|p| p.name == "RING_SELECTOR").unwrap();
        assert_eq!(selector.kind.max(), Some(3.0), "the selector's own max must track live COUNT, never a fixed ceiling");
    }

    #[test]
    fn ring_selector_rejects_an_index_past_the_live_count() {
        let mut ring = Ring::new(); // COUNT defaults to 1
        let err = ring.set_parameter("RING_SELECTOR", Value::Number(2.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn each_ring_can_be_given_its_own_colour() {
        let mut ring = Ring::new();
        ring.set_parameter("COUNT", Value::Number(2.0)).unwrap();

        ring.set_parameter("RING_SELECTOR", Value::Number(1.0)).unwrap();
        ring.set_parameter("RING_COLOR", Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 })).unwrap();

        ring.set_parameter("RING_SELECTOR", Value::Number(2.0)).unwrap();
        ring.set_parameter("RING_COLOR", Value::Color(Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 })).unwrap();

        assert_eq!(ring.colors[0], Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        assert_eq!(ring.colors[1], Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 });
    }

    #[test]
    fn growing_count_fills_new_rings_with_the_last_rings_colour() {
        let mut ring = Ring::new();
        ring.set_parameter("RING_COLOR", Value::Color(Color { r: 0.2, g: 0.4, b: 0.6, a: 1.0 })).unwrap();
        ring.set_parameter("COUNT", Value::Number(3.0)).unwrap();

        assert_eq!(ring.colors.len(), 3);
        assert_eq!(ring.colors[1], Color { r: 0.2, g: 0.4, b: 0.6, a: 1.0 });
        assert_eq!(ring.colors[2], Color { r: 0.2, g: 0.4, b: 0.6, a: 1.0 });
    }

    #[test]
    fn shrinking_count_clamps_an_out_of_range_selection() {
        let mut ring = Ring::new();
        ring.set_parameter("COUNT", Value::Number(3.0)).unwrap();
        ring.set_parameter("RING_SELECTOR", Value::Number(3.0)).unwrap();

        ring.set_parameter("COUNT", Value::Number(1.0)).unwrap();
        assert_eq!(ring.selected_ring, 1, "selection must be clamped back into range, not left dangling");
    }

    #[test]
    fn set_parameter_rejects_a_negative_radius() {
        let mut ring = Ring::new();
        let err = ring.set_parameter("RADIUS", Value::Number(-1.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn ring_in_graph_is_valid() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, RenderExecutor};

        let mut graph = Graph::new(8, 8);
        let ring_id = graph.add_node(Box::new(Ring::new()));
        graph.validate().expect("unwired ring is valid");
        RenderExecutor::new()
            .execute(&graph, ring_id, &context(8, 8))
            .expect("unwired ring renders");
    }

    // --- WebGPU Phase 1.2 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let ring = Ring::new();
        assert!(!ring.is_live());

        *ring.pending.borrow_mut() = Some(RingFingerprint {
            width: 8,
            height: 8,
            count: 1,
            radius_bits: 2.0f64.to_bits(),
            spacing_bits: 1.0f64.to_bits(),
            thickness_bits: 1.0f64.to_bits(),
            colors: vec![Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }],
        });
        assert!(ring.is_live());

        *ring.pending.borrow_mut() = None;
        assert!(!ring.is_live());
    }

    #[test]
    fn gpu_ring_matches_cpu_within_tolerance_once_warmed_up() {
        let Ok(gpu) = pollster::block_on(crate::gpu::GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };
        let gpu = Arc::new(gpu);

        let width = 8;
        let height = 8;

        let mut cpu_ring = Ring::new();
        cpu_ring.set_parameter("COUNT", Value::Number(2.0)).unwrap();
        cpu_ring.radius = 3.0;
        cpu_ring.spacing = 1.0;
        cpu_ring.thickness = 1.0;
        cpu_ring.colors[0] = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        cpu_ring.colors[1] = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
        let cpu_values = cpu_ring.execute(&context(width, height), &[]).unwrap();
        let Value::Image(cpu_result) = &cpu_values[0] else { panic!("expected an image") };

        let mut gpu_ring = Ring::new();
        gpu_ring.set_parameter("COUNT", Value::Number(2.0)).unwrap();
        gpu_ring.radius = 3.0;
        gpu_ring.spacing = 1.0;
        gpu_ring.thickness = 1.0;
        gpu_ring.colors[0] = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        gpu_ring.colors[1] = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };
        let _ = gpu_ring.execute(&gpu_ctx, &[]).unwrap();
        let gpu_values = gpu_ring.execute(&gpu_ctx, &[]).unwrap();
        let Value::Image(gpu_result) = &gpu_values[0] else { panic!("expected an image") };

        assert_eq!(cpu_result.pixels, gpu_result.pixels);
    }
}
