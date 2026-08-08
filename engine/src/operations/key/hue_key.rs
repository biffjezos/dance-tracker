// src/operations/key/hue_key.rs
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
use crate::graphics::{Color, FloatImage};
use crate::gpu::GpuState;

// GPU compute shader - SPECwebgpuoperations.md's Phase 1.3. Two input
// buffers (SOURCE/REFERENCE, not Foreground/Background). `target_hue` is
// computed once, CPU-side, via the already-tested `Color::to_hsv()`
// (reused directly rather than re-porting the full RGB->HSV conversion
// into WGSL a second time - HUE_KEY only ever needs the single hue
// scalar, not the full HSV triple) and passed down as a plain uniform
// float, same pattern as CHROMAKEY's KEY_COLOR/THRESHOLD. `hue_distance`
// itself IS ported to WGSL, since it runs per-pixel against REFERENCE's
// buffer. Its `%` usage doesn't need RGB_TO_HSV's floor-mod emulation:
// the dividend (`abs(a - b)`) is always non-negative by construction, so
// WGSL's truncated `%` and a euclidean one agree exactly here - see
// hue_distance's own doc comment on the CPU side for why.
const HUE_KEY_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> source: array<f32>;
    @group(0) @binding(1) var<storage, read> reference: array<f32>;
    @group(0) @binding(2) var<storage, read_write> output: array<f32>;
    @group(0) @binding(3) var<uniform> params: vec4<u32>;

    fn hue_distance(a: f32, b: f32) -> f32 {
        let diff = abs(a - b) % 360.0;
        let shortest = min(diff, 360.0 - diff);
        return shortest / 180.0;
    }

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params.x;
        let height = params.y;
        let target_hue = bitcast<f32>(params.z);
        let threshold = bitcast<f32>(params.w);

        if (id.x >= width || id.y >= height) {
            return;
        }

        let idx = (id.y * width + id.x) * 4u;
        let reference_hue = reference[idx] * 360.0;
        let distance = hue_distance(reference_hue, target_hue);

        output[idx] = source[idx];
        output[idx + 1u] = source[idx + 1u];
        output[idx + 2u] = source[idx + 2u];
        output[idx + 3u] = select(source[idx + 3u], 0.0, distance <= threshold);
    }
"#;

struct HueKeyGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_hue_key_pipeline(gpu: &GpuState) -> HueKeyGpuPipeline {
    let shader = gpu.create_shader(HUE_KEY_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("hue_key bind group layout"),
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

    let pipeline = gpu.create_compute_pipeline("hue_key pipeline", &shader, "main", &[&bind_group_layout]);

    HueKeyGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct HueKeyFingerprint {
    source: Value,
    reference: Value,
    target_hue_bits: u64,
    threshold_bits: u64,
}

impl HueKeyFingerprint {
    fn matches(&self, other: &HueKeyFingerprint) -> bool {
        self.target_hue_bits == other.target_hue_bits
            && self.threshold_bits == other.threshold_bits
            && value_ptr_eq(&self.source, &other.source)
            && value_ptr_eq(&self.reference, &other.reference)
    }
}

struct CompletedHueKeyJob {
    fingerprint: HueKeyFingerprint,
    pixels: Vec<f32>,
    width: u32,
    height: u32,
}

/// What an unconnected SOURCE shows - same convention as CHROMA KEY: a
/// flat, obviously-fake placeholder rather than the usual missing()
/// checker, since this is a mask-producing node with nothing to key
/// "removal" against.
const PLACEHOLDER_COLOR: Color = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };

/// Hue-based key: cuts SOURCE's alpha to 0 wherever REFERENCE's own hue
/// (its packed RGB TO HSV output - hue in the red channel, 0..360 mapped to
/// 0..255) is within THRESHOLD of HUE_COLOR's own hue, measured the short
/// way around the color wheel (350 degrees and 10 degrees are 20 degrees
/// apart, not 340). SOURCE's RGB and any already-lower alpha are otherwise
/// untouched.
///
/// Unlike CHROMA KEY, the signal being measured (REFERENCE) doesn't have
/// to be the same image as the content being keyed (SOURCE) - typically
/// both are wired from the same footage (REFERENCE via RGB TO HSV), but
/// comparing hue alone, independent of brightness/saturation, means a
/// screen that's unevenly lit doesn't need a threshold wide enough to
/// also catch unrelated dark, desaturated content the way CHROMA KEY's
/// raw RGB distance does.
pub struct HueKey {
    pub hue_color: Color,
    pub threshold: f64,
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale.
    gpu_pipeline: RefCell<Option<HueKeyGpuPipeline>>,
    pending: Rc<RefCell<Option<HueKeyFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedHueKeyJob>>>,
}

impl HueKey {
    pub fn new() -> Self {
        Self {
            hue_color: Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
            threshold: 0.1,
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Blur::dispatch_gpu` in structure, with two input images
    /// (SOURCE/REFERENCE) instead of one.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: HueKeyFingerprint, source: FloatImage, reference: FloatImage) {
        let width = source.width;
        let height = source.height;
        let len = source.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_hue_key_pipeline(&gpu));
            }
        }

        let source_buffer = gpu.upload("hue_key source", &source.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let reference_buffer = gpu.upload("hue_key reference", &reference.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "hue_key output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let target_hue_bits = (f64::from_bits(fingerprint.target_hue_bits) as f32).to_bits();
        let threshold_bits = (f64::from_bits(fingerprint.threshold_bits) as f32).to_bits();
        let params: [u32; 4] = [width, height, target_hue_bits, threshold_bits];
        let params_buffer = gpu.upload("hue_key params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "hue_key readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("hue_key bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: source_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: reference_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("hue_key dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pixels = gpu.read_buffer_blocking(&readback_buffer, len);
            *self.last_gpu_result.borrow_mut() = Some(CompletedHueKeyJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pixels = gpu.read_buffer_async(&readback_buffer, len).await;
                *last_gpu_result.borrow_mut() = Some(CompletedHueKeyJob {
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

    /// Shortest distance between two hues (degrees), normalized to 0..1
    /// (180 degrees - the maximum possible - is 1.0).
    fn hue_distance(a: f64, b: f64) -> f64 {
        let diff = (a - b).abs() % 360.0;
        let shortest = diff.min(360.0 - diff);
        shortest / 180.0
    }

    /// `reference` supplies the hue to compare (its own red channel,
    /// normalized 0.0..1.0, as packed by RGB TO HSV); `source` supplies
    /// the RGB and alpha that actually get keyed and returned. Same
    /// length required.
    pub fn key_pixels(source: &[f32], reference: &[f32], target_hue: f64, threshold: f64) -> Vec<f32> {
        let mut output = vec![0f32; source.len()];

        for ((src, reference_px), target) in source
            .chunks_exact(4)
            .zip(reference.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            let reference_hue = reference_px[0] as f64 * 360.0;
            let distance = Self::hue_distance(reference_hue, target_hue);

            target[0] = src[0];
            target[1] = src[1];
            target[2] = src[2];
            target[3] = if distance <= threshold { 0.0 } else { src[3] };
        }

        output
    }

    /// The keyed value of a single pixel, computed directly from
    /// `source`/`reference` - identical math to `key_pixels`'s own loop
    /// body for that index. Used by `execute()`'s bbox-restricted path
    /// (Phase 3 of BBOX_CONVENTIONS.md).
    fn key_single_pixel(source: &[f32], reference: &[f32], target_hue: f64, threshold: f64, x: u32, y: u32, width: u32) -> [f32; 4] {
        let idx = ((y * width + x) * 4) as usize;
        let reference_hue = reference[idx] as f64 * 360.0;
        let distance = Self::hue_distance(reference_hue, target_hue);

        [
            source[idx],
            source[idx + 1],
            source[idx + 2],
            if distance <= threshold { 0.0 } else { source[idx + 3] },
        ]
    }
}

impl Default for HueKey {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for HueKey {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "hue_key",
            menu: "KEY",
            label: "HUE KEY",
            action: None,
            ui_action: None,
            create_node: Some("hue_key"),
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
            display_name: "Hue Key",
            category: OperationCategory::Mask,
            inputs: vec![
                InputDescriptor { kind: Input::Source, accepts: PIXEL_KINDS },
                // Unlike PATCH's REFERENCE (a Number source), HueKey's
                // REFERENCE is a pixel source - rgb_to_hsv's FloatImage
                // output is what gets wired here (see PARKED_WORK.md).
                InputDescriptor { kind: Input::Reference, accepts: PIXEL_KINDS },
                InputDescriptor { kind: Input::Mask, accepts: PIXEL_KINDS },
            ],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "HUE_COLOR",
                kind: ParameterKind::Color,
                group: None,
            },
            ParameterDescriptor {
                name: "THRESHOLD",
                kind: ParameterKind::Number { step: 0.01, min: Some(0.0), max: Some(1.0) },
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "HUE_COLOR" => Some(Value::Color(self.hue_color)),
            "THRESHOLD" => Some(Value::Number(self.threshold)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("HUE_COLOR", Value::Color(color)) => {
                self.hue_color = color;
                Ok(())
            }
            ("THRESHOLD", Value::Number(v)) => {
                if !(0.0..=1.0).contains(&v) {
                    return Err(OperationError::InvalidParameterValue(
                        name.to_string(),
                        v.to_string(),
                    ));
                }
                self.threshold = v;
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
        let Some(source) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::solid(PLACEHOLDER_COLOR, ctx.meta.width, ctx.meta.height))]);
        };

        // No REFERENCE wired yet: nothing to key against, so SOURCE passes
        // through unchanged - same "not wired = no-op" convention MASK
        // already uses, not an error.
        let Some(reference) = find_input(inputs, Input::Reference) else {
            return Ok(vec![source.clone()]);
        };
        let reference_image = FloatImage::from_value(reference, ctx)?;

        let source_image = FloatImage::from_value(source, ctx)?;
        if source_image.width != reference_image.width || source_image.height != reference_image.height {
            return Err(OperationError::InvalidInputType(format!(
                "HUE KEY's REFERENCE is {}x{}, but SOURCE is {}x{}",
                reference_image.width, reference_image.height, source_image.width, source_image.height
            )));
        }

        let (target_hue, _, _) = self.hue_color.to_hsv();

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        // Phase 3 of BBOX_CONVENTIONS.md: with a MASK wired, apply_mask
        // below blends every pixel outside the relevant region straight
        // back to SOURCE anyway - so restrict the actual keying compute
        // to the intersection of MASK's own reported box and SOURCE's
        // own reported box (no growth needed - key_pixels reads only the
        // pixel it writes, no neighbors). SOURCE's box is a valid
        // operand here, same as CHROMA KEY: key_pixels is zero-preserving
        // in SOURCE alone - RGB always copies SOURCE unconditionally, and
        // alpha is either explicitly zeroed or left at SOURCE's own
        // (already-zero) alpha, regardless of REFERENCE's value. REFERENCE's
        // own reported box deliberately plays no role in the restriction -
        // it only decides *which* branch alpha takes, not whether the
        // result is zero when SOURCE already is; its full pixel buffer is
        // always read directly wherever needed, unrestricted.
        if let Some(mask) = &mask {
            let natural_box = find_bbox(&ctx.input_bboxes, Input::Source)
                .unwrap_or_else(|| Rect::full(source_image.width, source_image.height));
            let mask_box = find_bbox(&ctx.input_bboxes, Input::Mask)
                .unwrap_or_else(|| Rect::full(source_image.width, source_image.height));
            let work_area = natural_box.intersect(&mask_box);

            let width = source_image.width;
            let source_pixels = &source_image.pixels;
            let reference_pixels = &reference_image.pixels;

            let keyed = crate::graphics::compute_within_bbox(width, source_image.height, work_area, source_pixels, |x, y| {
                Self::key_single_pixel(source_pixels, reference_pixels, target_hue, self.threshold, x, y, width)
            });
            let keyed = crate::graphics::apply_mask(&source_image.pixels, keyed, Some(mask), width, source_image.height)?;

            return Ok(vec![Value::FloatImage(Arc::new(FloatImage { pixels: keyed, width, height: source_image.height }))]);
        }

        // Unmasked path: try GPU first when available.
        if let Some(gpu) = ctx.gpu.clone() {
            let fingerprint = HueKeyFingerprint {
                source: source.clone(),
                reference: reference.clone(),
                target_hue_bits: target_hue.to_bits(),
                threshold_bits: self.threshold.to_bits(),
            };

            let cached = self.last_gpu_result.borrow().as_ref()
                .filter(|completed| completed.fingerprint.matches(&fingerprint))
                .map(|completed| FloatImage { pixels: completed.pixels.clone(), width: completed.width, height: completed.height });

            if let Some(result) = cached {
                return Ok(vec![Value::FloatImage(Arc::new(result))]);
            }

            let already_pending = self.pending.borrow().as_ref().is_some_and(|p| p.matches(&fingerprint));
            if !already_pending {
                self.dispatch_gpu(gpu, fingerprint, source_image.clone(), reference_image.clone());
            }
        }

        let keyed = Self::key_pixels(&source_image.pixels, &reference_image.pixels, target_hue, self.threshold);

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: keyed,
            width: source_image.width,
            height: source_image.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(HueKey::new())
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
    fn a_matching_hue_is_keyed_out_regardless_of_brightness() {
        let hue_key = HueKey::new(); // default HUE_COLOR is pure green (120 deg)

        // Two very differently-lit patches of "green screen": bright green
        // and a dim, shadowed green. Both must key out.
        let bright_source = float_pixels(vec![0, 255, 0, 255]);
        let bright_reference = RgbToHsvHue::pack(120.0, 1.0, 1.0);

        let dim_source = float_pixels(vec![0, 60, 0, 255]);
        let dim_reference = RgbToHsvHue::pack(120.0, 1.0, 60.0 / 255.0);

        let bright_out = HueKey::key_pixels(&bright_source, &bright_reference, 120.0, hue_key.threshold);
        let dim_out = HueKey::key_pixels(&dim_source, &dim_reference, 120.0, hue_key.threshold);

        assert_eq!(bright_out[3], 0.0, "bright green must key out");
        assert_eq!(dim_out[3], 0.0, "dim, shadowed green (same hue) must key out too");
    }

    #[test]
    fn a_dark_but_different_hue_is_not_keyed_out() {
        // A dark, desaturated shirt - very different hue from green, even
        // though it's dim like the shadowed screen above.
        let source = float_pixels(vec![20, 20, 25, 255]);
        let reference = RgbToHsvHue::pack(240.0, 20.0 / 255.0, 25.0 / 255.0); // bluish hue

        let out = HueKey::key_pixels(&source, &reference, 120.0, 0.1);

        assert_eq!(out[3], 1.0, "a differently-hued dark pixel must not be keyed out just for being dark");
    }

    #[test]
    fn unconnected_hue_key_is_solid_by_default_not_the_missing_checker() {
        let hue_key = HueKey::new();
        let values = hue_key.execute(&context(2, 1), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, vec![255, 0, 255, 255, 255, 0, 255, 255]),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn source_without_reference_passes_through_unchanged() {
        let hue_key = HueKey::new();
        let source = image(vec![0, 255, 0, 255], 1, 1);

        let values = hue_key
            .execute(&context(1, 1), &[(Input::Source, Value::Image(source.clone()))])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, source.pixels),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn mismatched_reference_size_errors_instead_of_being_silently_ignored() {
        let hue_key = HueKey::new();
        let source = image(vec![0, 255, 0, 255], 1, 1);
        let reference = image(vec![85, 255, 255, 255, 85, 255, 255, 255], 2, 1);

        let err = hue_key
            .execute(&context(1, 1), &[
                (Input::Source, Value::Image(source)),
                (Input::Reference, Value::Image(reference)),
            ])
            .unwrap_err();

        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn wired_through_the_graph_with_rgb_to_hsv_feeding_reference() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor};
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::RgbToHsv;

        let mut graph = Graph::new(1, 1);

        let mut source_op = ImageSource::new();
        source_op.set_image(image(vec![0, 255, 0, 255], 1, 1));
        let source_id = graph.add_node(Box::new(source_op));

        let hsv_id = graph.add_node(Box::new(RgbToHsv::new()));
        graph.connect(hsv_id, Input::Source, source_id).unwrap();

        let key_id = graph.add_node(Box::new(HueKey::new()));
        graph.connect(key_id, Input::Source, source_id).unwrap();
        graph.connect(key_id, Input::Reference, hsv_id).unwrap();

        let values = PreviewExecutor::default()
            .execute(&graph, key_id, &context(1, 1))
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), vec![0, 255, 0, 0], "pure green must key out via RGB TO HSV -> HUE KEY");
    }

    #[test]
    fn reference_input_accepts_differs_from_patchs_reference() {
        // Proves accepts is per-operation, not per-Input-variant: Reference
        // means "Number source" on PATCH, "pixel source" here.
        use crate::operations::compose::Patch;

        let hue_key_metadata = HueKey::new().metadata();
        let hue_key_reference = hue_key_metadata.inputs.iter().find(|d| d.kind == Input::Reference).unwrap();

        let patch_metadata = Patch::new().metadata();
        let patch_reference = patch_metadata.inputs.iter().find(|d| d.kind == Input::Reference).unwrap();

        assert_ne!(hue_key_reference.accepts, patch_reference.accepts);
        assert!(hue_key_reference.accepts.contains(&OutputKind::FloatImage));
    }

    #[test]
    fn consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one() {
        let hue_key = HueKey::new(); // default HUE_COLOR is pure green, threshold 0.1

        let source = image(
            vec![
                255, 0, 0, 255,   0, 255, 0, 255,
                0, 255, 0, 255,   10, 20, 30, 255,
                255, 255, 255, 255, 0, 0, 0, 255,
            ],
            6, 1,
        );
        let reference = image(
            (0..6).flat_map(|_| RgbToHsvHue::pack(120.0, 1.0, 1.0).into_iter().map(|v| (v * 255.0) as u8).collect::<Vec<u8>>())
                .collect(),
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
            (Input::Reference, Value::Image(reference)),
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

        let restricted = hue_key.execute(&ctx_with_real_box, &inputs).unwrap();
        let unrestricted = hue_key.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box() {
        // Verifies key_pixels's zero-preservation in SOURCE directly (same
        // shape as CHROMA KEY): RGB always copies SOURCE unconditionally
        // and alpha is either explicitly zeroed or left at SOURCE's own
        // (already-zero) alpha, regardless of REFERENCE - SOURCE's own
        // box is therefore a valid intersection operand, and REFERENCE's
        // own box correctly plays no role at all.
        let hue_key = HueKey::new();

        let mut source_pixels = vec![0u8; 10 * 4];
        // Real content inside [3,7): pure green (keys out) at x=4, red
        // (stays) at x=5.
        source_pixels[3 * 4..3 * 4 + 4].copy_from_slice(&[0, 255, 0, 255]);
        source_pixels[4 * 4..4 * 4 + 4].copy_from_slice(&[0, 255, 0, 255]);
        source_pixels[5 * 4..5 * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
        source_pixels[6 * 4..6 * 4 + 4].copy_from_slice(&[0, 255, 0, 255]);
        let source = image(source_pixels, 10, 1);

        // REFERENCE reports pure green's hue everywhere (so the pure-green
        // SOURCE pixels key out, the red one doesn't).
        let reference_pixel = RgbToHsvHue::pack(120.0, 1.0, 1.0);
        let reference = image(
            (0..10).flat_map(|_| reference_pixel.iter().map(|&v| (v * 255.0) as u8).collect::<Vec<u8>>()).collect(),
            10, 1,
        );
        let mask = image(vec![255; 10 * 4], 10, 1);

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Reference, Value::Image(reference)),
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

        let restricted = hue_key.execute(&ctx_with_real_source_box, &inputs).unwrap();
        let unrestricted = hue_key.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one() {
        use crate::graphics::mask::{reset_pixels_computed, take_pixels_computed};

        let hue_key = HueKey::new();

        let source = image((0..16).flat_map(|n| [n, n, n, 255]).collect(), 4, 4);
        let reference_pixel = RgbToHsvHue::pack(120.0, 1.0, 1.0);
        let reference = image(
            (0..16).flat_map(|_| reference_pixel.iter().map(|&v| (v * 255.0) as u8).collect::<Vec<u8>>()).collect(),
            4, 4,
        );
        let mask = image(vec![255; 4 * 4 * 4], 4, 4);

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Reference, Value::Image(reference)),
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
        hue_key.execute(&small_box_ctx, &inputs).unwrap();
        let small_box_pixels = take_pixels_computed().expect("HUE KEY with a wired MASK must record a pixel count");

        let full_frame_ctx = context(4, 4);
        reset_pixels_computed();
        hue_key.execute(&full_frame_ctx, &inputs).unwrap();
        let full_frame_pixels = take_pixels_computed().expect("HUE KEY with a wired MASK must record a pixel count");

        assert_eq!(small_box_pixels, 1);
        assert_eq!(full_frame_pixels, 16);
        assert!(small_box_pixels < full_frame_pixels);
    }

    #[test]
    fn checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor, RenderExecutor};
        use crate::graphics::Color;
        use crate::operations::generators::Checkerboard;
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::{Move, Resize};

        let mut graph = Graph::new(4, 4);

        let mut source = ImageSource::new();
        source.set_image(image((0..16).flat_map(|n| [n * 15, 0, 0, 255]).collect(), 4, 4));
        let source_id = graph.add_node(Box::new(source));

        let mut reference_source = ImageSource::new();
        let reference_pixel = RgbToHsvHue::pack(120.0, 1.0, 1.0);
        reference_source.set_image(image(
            (0..16).flat_map(|_| reference_pixel.iter().map(|&v| (v * 255.0) as u8).collect::<Vec<u8>>()).collect(),
            4, 4,
        ));
        let reference_id = graph.add_node(Box::new(reference_source));

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

        let hue_key_id = graph.add_node(Box::new(HueKey::new()));
        graph.connect(hue_key_id, Input::Source, source_id).unwrap();
        graph.connect(hue_key_id, Input::Reference, reference_id).unwrap();
        graph.connect(hue_key_id, Input::Mask, move_id).unwrap();

        graph.validate().expect("the wired pipeline is valid");

        let ctx = context(4, 4);

        let on_values = RenderExecutor::new().execute(&graph, hue_key_id, &ctx).unwrap();
        let on_pixels = as_u8_pixels(&on_values[0]);

        let source_value = PreviewExecutor::default().execute(&graph, source_id, &ctx).unwrap().into_iter().next().unwrap();
        let reference_value = PreviewExecutor::default().execute(&graph, reference_id, &ctx).unwrap().into_iter().next().unwrap();
        let mask_value = PreviewExecutor::default().execute(&graph, move_id, &ctx).unwrap().into_iter().next().unwrap();

        let hue_key_off = HueKey::new();
        let off_values = hue_key_off.execute(&ctx, &[
            (Input::Source, source_value),
            (Input::Reference, reference_value),
            (Input::Mask, mask_value),
        ]).unwrap();
        let off_pixels = as_u8_pixels(&off_values[0]);

        assert_eq!(on_pixels, off_pixels, "bbox consumption on vs off must produce pixel-identical output");
    }

    // --- WebGPU Phase 1.3 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let hue_key = HueKey::new();
        assert!(!hue_key.is_live());

        *hue_key.pending.borrow_mut() = Some(HueKeyFingerprint {
            source: Value::Number(0.0),
            reference: Value::Number(0.0),
            target_hue_bits: 120.0f64.to_bits(),
            threshold_bits: 0.1f64.to_bits(),
        });
        assert!(hue_key.is_live());

        *hue_key.pending.borrow_mut() = None;
        assert!(!hue_key.is_live());
    }

    #[test]
    fn gpu_hue_key_matches_cpu_within_tolerance_once_warmed_up() {
        let Ok(gpu) = pollster::block_on(crate::gpu::GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };
        let gpu = Arc::new(gpu);

        let width = 6;
        let height = 4;
        let source_pixels: Vec<u8> = (0..(width * height))
            .flat_map(|n| {
                let v = ((n * 43) % 256) as u8;
                [v, v.wrapping_add(70), v.wrapping_add(140), 255]
            })
            .collect();
        let source = image(source_pixels, width, height);

        // Reference hues alternating between pure green (120deg, keys
        // out against the default HUE_COLOR) and pure blue (240deg,
        // doesn't) - exercises both branches of the select().
        let reference_pixels: Vec<u8> = (0..(width * height))
            .flat_map(|n| {
                let hue = if n % 2 == 0 { 120.0 } else { 240.0 };
                RgbToHsvHue::pack(hue, 1.0, 1.0).into_iter().map(|v| (v * 255.0) as u8).collect::<Vec<u8>>()
            })
            .collect();
        let reference = image(reference_pixels, width, height);

        let cpu_hue_key = HueKey::new();
        let cpu_values = cpu_hue_key
            .execute(&context(width, height), &[
                (Input::Source, Value::Image(source.clone())),
                (Input::Reference, Value::Image(reference.clone())),
            ])
            .unwrap();
        let Value::FloatImage(cpu_result) = &cpu_values[0] else { panic!("expected a float image") };

        let gpu_hue_key = HueKey::new();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };
        let _ = gpu_hue_key.execute(&gpu_ctx, &[
            (Input::Source, Value::Image(source.clone())),
            (Input::Reference, Value::Image(reference.clone())),
        ]).unwrap();
        let gpu_values = gpu_hue_key.execute(&gpu_ctx, &[
            (Input::Source, Value::Image(source)),
            (Input::Reference, Value::Image(reference)),
        ]).unwrap();
        let Value::FloatImage(gpu_result) = &gpu_values[0] else { panic!("expected a float image") };

        assert_eq!(cpu_result.pixels.len(), gpu_result.pixels.len());
        for (index, (cpu_px, gpu_px)) in cpu_result.pixels.iter().zip(gpu_result.pixels.iter()).enumerate() {
            assert!((cpu_px - gpu_px).abs() < 1e-4, "channel {}: cpu={}, gpu={}", index, cpu_px, gpu_px);
        }
    }

    /// Test-only helper for building a synthetic "RGB TO HSV output"
    /// pixel directly from a known hue/saturation/value, without going
    /// through an actual RGB source and the real conversion - keeps the
    /// hue-distance tests above focused on HueKey's own math.
    struct RgbToHsvHue;
    impl RgbToHsvHue {
        fn pack(hue_degrees: f64, saturation: f64, value: f64) -> Vec<f32> {
            vec![(hue_degrees / 360.0) as f32, saturation as f32, value as f32, 1.0]
        }
    }
}
