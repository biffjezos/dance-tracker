// src/operations/transform/shuffle.rs
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::compositor::{
    bbox::Rect,
    Context,
    OperationError,
    Input,
    input::{find_bbox, find_input},
    Operation,
    OperationDescriptor,
    metadata::{ InputDescriptor, OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind, PIXEL_KINDS },
    value::value_ptr_eq,
    Value
};
use crate::graphics::FloatImage;
use crate::gpu::GpuState;
use std::sync::Arc;

// GPU compute shader - SPECwebgpuoperations.md's Phase 1.1. Only the
// unmasked path is GPU-accelerated (the blanket rule). Unlike BLUR/
// INVERT/RGB_TO_HSV's single vec4<u32> uniform, SHUFFLE needs six u32
// values (width, height, four channel selectors) - `array<vec4<u32>, 2>`
// is the WGSL-uniform-safe way to fit that: each element is a full
// 16-byte vec4 (size == align == stride), so the "array stride must be a
// multiple of 16 bytes" uniform-address-space rule is met trivially, with
// no manual padding - unlike a hypothetical `array<u32, 6>` uniform,
// which would need each individual u32 padded to 16 bytes.
const SHUFFLE_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> input: array<f32>;
    @group(0) @binding(1) var<storage, read_write> output: array<f32>;
    @group(0) @binding(2) var<uniform> params: array<vec4<u32>, 2>;

    fn channel_value(pixel: vec4<f32>, sel: u32) -> f32 {
        if (sel == 0u) { return pixel.x; }
        if (sel == 1u) { return pixel.y; }
        if (sel == 2u) { return pixel.z; }
        if (sel == 3u) { return pixel.w; }
        return 0.0;
    }

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params[0].x;
        let height = params[0].y;
        let red_sel = params[0].z;
        let green_sel = params[0].w;
        let blue_sel = params[1].x;
        let alpha_sel = params[1].y;

        if (id.x >= width || id.y >= height) {
            return;
        }

        let idx = (id.y * width + id.x) * 4u;
        let pixel = vec4<f32>(input[idx], input[idx + 1u], input[idx + 2u], input[idx + 3u]);

        output[idx] = channel_value(pixel, red_sel);
        output[idx + 1u] = channel_value(pixel, green_sel);
        output[idx + 2u] = channel_value(pixel, blue_sel);
        output[idx + 3u] = channel_value(pixel, alpha_sel);
    }
"#;

struct ShuffleGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_shuffle_pipeline(gpu: &GpuState) -> ShuffleGpuPipeline {
    let shader = gpu.create_shader(SHUFFLE_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shuffle bind group layout"),
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

    let pipeline = gpu.create_compute_pipeline("shuffle pipeline", &shader, "main", &[&bind_group_layout]);

    ShuffleGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct ShuffleFingerprint {
    source: Value,
    red: ShuffleChannel,
    green: ShuffleChannel,
    blue: ShuffleChannel,
    alpha: ShuffleChannel,
}

impl ShuffleFingerprint {
    fn matches(&self, other: &ShuffleFingerprint) -> bool {
        self.red == other.red
            && self.green == other.green
            && self.blue == other.blue
            && self.alpha == other.alpha
            && value_ptr_eq(&self.source, &other.source)
    }
}

struct CompletedShuffleJob {
    fingerprint: ShuffleFingerprint,
    pixels: Vec<f32>,
    width: u32,
    height: u32,
}

/// Channel selection for Shuffle operation.
/// OFF writes 0 into the target channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShuffleChannel {
    R,
    G,
    B,
    A,
    Off,
}

/// The complete set of values a channel selector may take - the single source
/// of truth for both parsing and what the UI is allowed to offer.
pub const SHUFFLE_CHANNELS: &[&str] = &[
    "RED",
    "GREEN",
    "BLUE",
    "ALPHA",
    "OFF",
];

impl Default for ShuffleChannel {
    fn default() -> Self {
        ShuffleChannel::R
    }
}

impl ShuffleChannel {
    pub fn to_str(&self) -> &'static str {
        match self {
            ShuffleChannel::R => "RED",
            ShuffleChannel::G => "GREEN",
            ShuffleChannel::B => "BLUE",
            ShuffleChannel::A => "ALPHA",
            ShuffleChannel::Off => "OFF",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "RED" | "R" => Some(ShuffleChannel::R),
            "GREEN" | "G" => Some(ShuffleChannel::G),
            "BLUE" | "B" => Some(ShuffleChannel::B),
            "ALPHA" | "A" => Some(ShuffleChannel::A),
            "OFF" => Some(ShuffleChannel::Off),
            _ => None,
        }
    }

    /// Selector index the GPU shader's `channel_value()` switches on -
    /// matches `channel_value`'s own field-index mapping exactly (R=0
    /// through A=3, same as this channel's own position in a packed RGBA
    /// pixel; Off=4 is unused input-side but harmless as its own branch).
    fn to_gpu_selector(self) -> u32 {
        match self {
            ShuffleChannel::R => 0,
            ShuffleChannel::G => 1,
            ShuffleChannel::B => 2,
            ShuffleChannel::A => 3,
            ShuffleChannel::Off => 4,
        }
    }
}

/// Shuffle operation - remaps RGBA channels
pub struct Shuffle {
    pub red: ShuffleChannel,
    pub green: ShuffleChannel,
    pub blue: ShuffleChannel,
    pub alpha: ShuffleChannel,
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale.
    gpu_pipeline: RefCell<Option<ShuffleGpuPipeline>>,
    pending: Rc<RefCell<Option<ShuffleFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedShuffleJob>>>,
}

impl Shuffle {
    pub fn new() -> Self {
        Self {
            red: ShuffleChannel::R,
            green: ShuffleChannel::G,
            blue: ShuffleChannel::B,
            alpha: ShuffleChannel::A,
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Blur::dispatch_gpu` exactly in structure.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: ShuffleFingerprint, source: FloatImage) {
        let width = source.width;
        let height = source.height;
        let len = source.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_shuffle_pipeline(&gpu));
            }
        }

        let input_buffer = gpu.upload("shuffle input", &source.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "shuffle output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let params: [u32; 8] = [
            width, height, fingerprint.red.to_gpu_selector(), fingerprint.green.to_gpu_selector(),
            fingerprint.blue.to_gpu_selector(), fingerprint.alpha.to_gpu_selector(), 0, 0,
        ];
        let params_buffer = gpu.upload("shuffle params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "shuffle readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("shuffle bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("shuffle dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pixels = gpu.read_buffer_blocking(&readback_buffer, len);
            *self.last_gpu_result.borrow_mut() = Some(CompletedShuffleJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pixels = gpu.read_buffer_async(&readback_buffer, len).await;
                *last_gpu_result.borrow_mut() = Some(CompletedShuffleJob {
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
    
    /// Get the value for one output channel out of a single RGBA pixel.
    /// Generic (u8 or f32) - shuffling is pure reordering/zeroing, no
    /// arithmetic on channel values, so there's nothing here that could
    /// ever produce or need to guard against an out-of-gamut result.
    fn channel_value<T: Copy + Default>(pixel: &[T], channel: ShuffleChannel) -> T {
        match channel {
            ShuffleChannel::R => pixel[0],
            ShuffleChannel::G => pixel[1],
            ShuffleChannel::B => pixel[2],
            ShuffleChannel::A => pixel[3],
            ShuffleChannel::Off => T::default(),
        }
    }

    /// Remap the channels of a packed RGBA buffer.
    /// Works on raw pixels so it applies to every pixel-bearing Value alike.
    pub fn shuffle_pixels<T: Copy + Default>(&self, pixels: &[T]) -> Vec<T> {
        let mut output = vec![T::default(); pixels.len()];

        for (source, target) in pixels
            .chunks_exact(4)
            .zip(output.chunks_exact_mut(4))
        {
            target[0] = Self::channel_value(source, self.red);
            target[1] = Self::channel_value(source, self.green);
            target[2] = Self::channel_value(source, self.blue);
            target[3] = Self::channel_value(source, self.alpha);
        }

        output
    }
}

impl Operation for Shuffle {
    
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "shuffle",
            menu: "TRANSFORM",
            label: "SHUFFLE",
            action: None,
            ui_action: None,
            create_node: Some("shuffle"),
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
            display_name: "Shuffle",
            category: OperationCategory::Color,
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
                name: "RED",
                kind: ParameterKind::Enum(SHUFFLE_CHANNELS),
                group: None,
            },
            ParameterDescriptor {
                name: "GREEN",
                kind: ParameterKind::Enum(SHUFFLE_CHANNELS),
                group: None,
            },
            ParameterDescriptor {
                name: "BLUE",
                kind: ParameterKind::Enum(SHUFFLE_CHANNELS),
                group: None,
            },
            ParameterDescriptor {
                name: "ALPHA",
                kind: ParameterKind::Enum(SHUFFLE_CHANNELS),
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "RED" => Some(Value::Text(self.red.to_str().to_string())),
            "GREEN" => Some(Value::Text(self.green.to_str().to_string())),
            "BLUE" => Some(Value::Text(self.blue.to_str().to_string())),
            "ALPHA" => Some(Value::Text(self.alpha.to_str().to_string())),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        if let Value::Text(s) = value {
            match name {
                "RED" => {
                    self.red = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "GREEN" => {
                    self.green = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "BLUE" => {
                    self.blue = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "ALPHA" => {
                    self.alpha = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                _ => return Err(OperationError::UnknownParameter(name.to_string())),
            }
            Ok(())
        } else {
            Err(OperationError::InvalidParameterType(name.to_string()))
        }
    }

    // See blur.rs's identical override for the full rationale.
    fn is_live(&self) -> bool {
        self.pending.borrow().is_some()
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        /*
        Nothing wired in yet: an unconnected SHUFFLE is a legal graph state,
        so it produces black at the current resolution rather than failing.
        */
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![
                Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))
            ]);
        };

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        // Phase 3 of BBOX_CONVENTIONS.md: with a MASK wired, apply_mask
        // below blends every pixel outside the relevant region straight
        // back to `original` anyway - so restrict the actual shuffle
        // compute to the intersection of MASK's own reported box and
        // SOURCE's own reported box (no growth needed, unlike BLUR -
        // SHUFFLE reads only the same pixel it writes, no neighbors).
        // SOURCE's box IS a valid operand here (unlike INVERT):
        // shuffle_pixels is zero-preserving - every output channel is
        // either a copy of a source channel or Off's T::default() (0), so
        // shuffling [0,0,0,0] always produces [0,0,0,0] regardless of the
        // channel mapping. Without a MASK, there's nothing to restrict
        // against, so the original full-frame path is used unchanged.
        if let Some(mask) = &mask {
            let source = FloatImage::from_value(value, ctx)?;
            let natural_box = find_bbox(&ctx.input_bboxes, Input::Source)
                .unwrap_or_else(|| Rect::full(source.width, source.height));
            let mask_box = find_bbox(&ctx.input_bboxes, Input::Mask)
                .unwrap_or_else(|| Rect::full(source.width, source.height));
            let work_area = natural_box.intersect(&mask_box);

            let width = source.width;
            let pixels = &source.pixels;

            let shuffled = crate::graphics::compute_within_bbox(width, source.height, work_area, pixels, |x, y| {
                let idx = ((y * width + x) * 4) as usize;
                let pixel = &pixels[idx..idx + 4];
                [
                    Self::channel_value(pixel, self.red),
                    Self::channel_value(pixel, self.green),
                    Self::channel_value(pixel, self.blue),
                    Self::channel_value(pixel, self.alpha),
                ]
            });
            let shuffled = crate::graphics::apply_mask(&source.pixels, shuffled, Some(mask), width, source.height)?;

            return Ok(vec![Value::FloatImage(Arc::new(FloatImage { pixels: shuffled, width, height: source.height }))]);
        }

        // Unmasked path: try GPU first when available - shuffling is pure
        // reordering/zeroing, no trivial-identity short-circuit worth
        // special-casing (an all-R->R,G->G,... identity mapping is a
        // legitimate, if unusual, wiring - not distinguished from any
        // other mapping).
        if let Some(gpu) = ctx.gpu.clone() {
            let fingerprint = ShuffleFingerprint {
                source: value.clone(),
                red: self.red,
                green: self.green,
                blue: self.blue,
                alpha: self.alpha,
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
                self.dispatch_gpu(gpu, fingerprint, source);
            }
        }

        let source = FloatImage::from_value(value, ctx)?;
        let shuffled = self.shuffle_pixels(&source.pixels);

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: shuffled,
            width: source.width,
            height: source.height,
        }))])
    }
}

// Inventory registration for Shuffle
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Shuffle::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::graph::Graph;
    use crate::compositor::executors::{Execute, RenderExecutor};
    use crate::graphics::{ImageFormat, U8Image};
    use crate::operations::sources::ImageSource;

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
    fn an_unconnected_shuffle_produces_the_missing_placeholder_at_the_current_resolution() {
        let shuffle = Shuffle::new();

        let values = shuffle
            .execute(&context(2, 1), &[])
            .expect("an unwired shuffle is a legal graph state");

        match &values[0] {
            Value::Image(output) => {
                assert_eq!(output.width, 2);
                assert_eq!(output.height, 1);
                // Both pixels fall in the same 16px checker tile at this
                // tiny size, so they're the placeholder's magenta, not black.
                assert_eq!(output.pixels, vec![255, 0, 255, 255, 255, 0, 255, 255]);
            }
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn channels_are_taken_from_the_selected_source_channel() {
        let mut shuffle = Shuffle::new();

        shuffle.set_parameter("RED", Value::Text("GREEN".into())).unwrap();
        shuffle.set_parameter("GREEN", Value::Text("BLUE".into())).unwrap();
        shuffle.set_parameter("BLUE", Value::Text("RED".into())).unwrap();
        shuffle.set_parameter("ALPHA", Value::Text("OFF".into())).unwrap();

        let input = Value::Image(image(vec![10, 20, 30, 40], 1, 1));

        let values = shuffle
            .execute(&context(1, 1), &[(Input::Source, input)])
            .expect("shuffle should accept an image");

        assert_eq!(as_u8_pixels(&values[0]), vec![20, 30, 10, 0]);
    }

    #[test]
    fn off_zeroes_a_channel_without_touching_the_others() {
        let mut shuffle = Shuffle::new();
        shuffle.set_parameter("GREEN", Value::Text("OFF".into())).unwrap();

        let input = Value::Image(image(vec![1, 2, 3, 4], 1, 1));

        let values = shuffle
            .execute(&context(1, 1), &[(Input::Source, input)])
            .expect("shuffle should accept an image");

        assert_eq!(as_u8_pixels(&values[0]), vec![1, 0, 3, 4]);
    }

    #[test]
    fn shuffle_accepts_an_out_of_gamut_float_image_and_preserves_the_values() {
        let shuffle = Shuffle::new();
        let input = Value::FloatImage(Arc::new(FloatImage { pixels: vec![1.5, -0.2, 0.5, 1.0], width: 1, height: 1 }));

        let values = shuffle
            .execute(&context(1, 1), &[(Input::Source, input)])
            .expect("shuffle should accept a float image");

        match &values[0] {
            Value::FloatImage(out) => assert_eq!(out.pixels, vec![1.5, -0.2, 0.5, 1.0]),
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn an_unconnected_shuffle_does_not_invalidate_the_graph() {
        let mut graph = Graph::new(4, 4);
        let shuffle_id = graph.add_node(Box::new(Shuffle::new()));

        graph.validate().expect("an unwired input is not a graph failure");

        RenderExecutor::new()
            .execute(&graph, shuffle_id, &context(4, 4))
            .expect("an unwired shuffle still renders");
    }

    #[test]
    fn a_source_wired_into_shuffle_is_read_through_the_graph() {
        let mut graph = Graph::new(1, 1);

        let mut source = ImageSource::new();
        source.set_image(image(vec![10, 20, 30, 40], 1, 1));

        let source_id = graph.add_node(Box::new(source));

        let mut shuffle = Shuffle::new();
        shuffle.set_parameter("RED", Value::Text("BLUE".into())).unwrap();

        let shuffle_id = graph.add_node(Box::new(shuffle));

        graph
            .connect(shuffle_id, Input::Source, source_id)
            .expect("a source should connect to shuffle");

        graph.validate().expect("a wired graph is valid");

        let values = RenderExecutor::new()
            .execute(&graph, shuffle_id, &context(1, 1))
            .expect("the wired graph should render");

        assert_eq!(as_u8_pixels(&values[0]), vec![30, 20, 30, 40]);
    }

    #[test]
    fn a_zero_alpha_mask_leaves_the_channels_unshuffled() {
        let mut shuffle = Shuffle::new();
        shuffle.set_parameter("RED", Value::Text("GREEN".into())).unwrap();

        let input = image(vec![10, 20, 30, 40], 1, 1);
        let mask = image(vec![0, 0, 0, 0], 1, 1);

        let values = shuffle
            .execute(&context(1, 1), &[
                (Input::Source, Value::Image(input.clone())),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), input.pixels);
    }

    #[test]
    fn consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one() {
        let mut shuffle = Shuffle::new();
        shuffle.set_parameter("RED", Value::Text("GREEN".into())).unwrap();

        let source = image((0..6).flat_map(|n| [n * 10, n * 10 + 1, n * 10 + 2, 255]).collect(), 6, 1);
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
                (Input::Source, Rect::full(6, 1)),
                (Input::Mask, Rect { x0: 2, y0: 0, x1: 4, y1: 1 }),
            ],
            ..context(6, 1)
        };
        let ctx_full_frame = context(6, 1);

        let restricted = shuffle.execute(&ctx_with_real_box, &inputs).unwrap();
        let unrestricted = shuffle.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box() {
        // Verifies SHUFFLE's zero-preservation claim directly rather than
        // assuming it: shuffle([0,0,0,0]) is always [0,0,0,0] regardless
        // of channel mapping (every option is either a copy of a source
        // channel or Off's default 0), so SOURCE's own box IS a valid
        // operand in work_area's intersection here - unlike INVERT.
        let mut shuffle = Shuffle::new();
        shuffle.set_parameter("RED", Value::Text("BLUE".into())).unwrap();

        let mut source_pixels = vec![0u8; 10 * 4];
        for x in 3..7 {
            source_pixels[x * 4..x * 4 + 4].copy_from_slice(&[100, 150, 200, 255]);
        }
        let source = image(source_pixels, 10, 1);
        let mask = image(vec![255; 10 * 4], 10, 1);

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let ctx_with_real_source_box = Context {
            input_bboxes: vec![
                (Input::Source, Rect { x0: 3, y0: 0, x1: 7, y1: 1 }),
                (Input::Mask, Rect::full(10, 1)),
            ],
            ..context(10, 1)
        };
        let ctx_full_frame = context(10, 1);

        let restricted = shuffle.execute(&ctx_with_real_source_box, &inputs).unwrap();
        let unrestricted = shuffle.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one() {
        use crate::graphics::mask::{reset_pixels_computed, take_pixels_computed};

        let shuffle = Shuffle::new();

        let source = image((0..16).flat_map(|n| [n, n, n, 255]).collect(), 4, 4);
        let mask = image(vec![255; 4 * 4 * 4], 4, 4);

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let small_box_ctx = Context {
            input_bboxes: vec![
                (Input::Source, Rect::full(4, 4)),
                (Input::Mask, Rect { x0: 1, y0: 1, x1: 2, y1: 2 }),
            ],
            ..context(4, 4)
        };
        reset_pixels_computed();
        shuffle.execute(&small_box_ctx, &inputs).unwrap();
        let small_box_pixels = take_pixels_computed().expect("SHUFFLE with a wired MASK must record a pixel count");

        let full_frame_ctx = context(4, 4);
        reset_pixels_computed();
        shuffle.execute(&full_frame_ctx, &inputs).unwrap();
        let full_frame_pixels = take_pixels_computed().expect("SHUFFLE with a wired MASK must record a pixel count");

        assert_eq!(small_box_pixels, 1);
        assert_eq!(full_frame_pixels, 16);
        assert!(small_box_pixels < full_frame_pixels);
    }

    #[test]
    fn checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off() {
        use crate::compositor::executors::PreviewExecutor;
        use crate::graphics::Color;
        use crate::operations::generators::Checkerboard;
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

        let mut shuffle = Shuffle::new();
        shuffle.set_parameter("RED", Value::Text("BLUE".into())).unwrap();
        let shuffle_id = graph.add_node(Box::new(shuffle));
        graph.connect(shuffle_id, Input::Source, source_id).unwrap();
        graph.connect(shuffle_id, Input::Mask, move_id).unwrap();

        graph.validate().expect("the wired pipeline is valid");

        let ctx = context(4, 4);

        let on_values = RenderExecutor::new().execute(&graph, shuffle_id, &ctx).unwrap();
        let on_pixels = as_u8_pixels(&on_values[0]);

        let source_value = PreviewExecutor::default().execute(&graph, source_id, &ctx).unwrap().into_iter().next().unwrap();
        let mask_value = PreviewExecutor::default().execute(&graph, move_id, &ctx).unwrap().into_iter().next().unwrap();

        let mut shuffle_off = Shuffle::new();
        shuffle_off.set_parameter("RED", Value::Text("BLUE".into())).unwrap();
        let off_values = shuffle_off.execute(&ctx, &[
            (Input::Source, source_value),
            (Input::Mask, mask_value),
        ]).unwrap();
        let off_pixels = as_u8_pixels(&off_values[0]);

        assert_eq!(on_pixels, off_pixels, "bbox consumption on vs off must produce pixel-identical output");
    }

    // --- WebGPU Phase 1.1 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let shuffle = Shuffle::new();
        assert!(!shuffle.is_live());

        *shuffle.pending.borrow_mut() = Some(ShuffleFingerprint {
            source: Value::Number(0.0),
            red: ShuffleChannel::R,
            green: ShuffleChannel::G,
            blue: ShuffleChannel::B,
            alpha: ShuffleChannel::Off,
        });
        assert!(shuffle.is_live());

        *shuffle.pending.borrow_mut() = None;
        assert!(!shuffle.is_live());
    }

    #[test]
    fn gpu_shuffle_matches_cpu_within_tolerance_once_warmed_up() {
        let Ok(gpu) = pollster::block_on(crate::gpu::GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };
        let gpu = Arc::new(gpu);

        let width = 5;
        let height = 3;
        let pixels: Vec<u8> = (0..(width * height))
            .flat_map(|n| {
                let v = ((n * 61) % 256) as u8;
                [v, v.wrapping_add(30), v.wrapping_add(90), v.wrapping_add(150)]
            })
            .collect();
        let input = image(pixels, width, height);

        let mut cpu_shuffle = Shuffle::new();
        cpu_shuffle.set_parameter("RED", Value::Text("BLUE".into())).unwrap();
        cpu_shuffle.set_parameter("GREEN", Value::Text("OFF".into())).unwrap();
        let cpu_values = cpu_shuffle
            .execute(&context(width, height), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();
        let cpu_result = as_u8_pixels(&cpu_values[0]);

        let mut gpu_shuffle = Shuffle::new();
        gpu_shuffle.set_parameter("RED", Value::Text("BLUE".into())).unwrap();
        gpu_shuffle.set_parameter("GREEN", Value::Text("OFF".into())).unwrap();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };
        let _ = gpu_shuffle.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        let gpu_values = gpu_shuffle.execute(&gpu_ctx, &[(Input::Source, Value::Image(input.clone()))]).unwrap();
        let gpu_result = as_u8_pixels(&gpu_values[0]);

        assert_eq!(cpu_result, gpu_result);
    }
}
