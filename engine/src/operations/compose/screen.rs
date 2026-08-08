// src/operations/compose/screen.rs
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
    metadata::{InputDescriptor, OperationCategory, OperationMetadata, OutputKind, PIXEL_KINDS},
    value::value_ptr_eq,
    Value,
};
use crate::graphics::FloatImage;
use crate::gpu::GpuState;

// GPU compute shader - SPECwebgpuoperations.md's Phase 1.3. Same shape
// as ADD's, see its own doc comment.
const SCREEN_SHADER: &str = r#"
    @group(0) @binding(0) var<storage, read> foreground: array<f32>;
    @group(0) @binding(1) var<storage, read> background: array<f32>;
    @group(0) @binding(2) var<storage, read_write> output: array<f32>;
    @group(0) @binding(3) var<uniform> params: vec4<u32>;

    fn screen_channel(a: f32, b: f32) -> f32 {
        return 1.0 - (1.0 - a) * (1.0 - b);
    }

    @compute @workgroup_size(8, 8, 1)
    fn main(@builtin(global_invocation_id) id: vec3<u32>) {
        let width = params.x;
        let height = params.y;

        if (id.x >= width || id.y >= height) {
            return;
        }

        let idx = (id.y * width + id.x) * 4u;
        output[idx] = screen_channel(foreground[idx], background[idx]);
        output[idx + 1u] = screen_channel(foreground[idx + 1u], background[idx + 1u]);
        output[idx + 2u] = screen_channel(foreground[idx + 2u], background[idx + 2u]);
        output[idx + 3u] = screen_channel(foreground[idx + 3u], background[idx + 3u]);
    }
"#;

struct ScreenGpuPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

fn build_screen_pipeline(gpu: &GpuState) -> ScreenGpuPipeline {
    let shader = gpu.create_shader(SCREEN_SHADER);

    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("screen bind group layout"),
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

    let pipeline = gpu.create_compute_pipeline("screen pipeline", &shader, "main", &[&bind_group_layout]);

    ScreenGpuPipeline { pipeline, bind_group_layout }
}

#[derive(Clone)]
struct ScreenFingerprint {
    foreground: Value,
    background: Value,
}

impl ScreenFingerprint {
    fn matches(&self, other: &ScreenFingerprint) -> bool {
        value_ptr_eq(&self.foreground, &other.foreground) && value_ptr_eq(&self.background, &other.background)
    }
}

struct CompletedScreenJob {
    fingerprint: ScreenFingerprint,
    pixels: Vec<f32>,
    width: u32,
    height: u32,
}

/// Screen operation - the inverse of Multiply (Screen(A,B) =
/// Invert(Multiply(Invert(A), Invert(B)))), computed directly rather than
/// through three passes. Unclamped, same as Multiply/Add/Subtract - see
/// their own doc comments for why. Both inputs accept a bounded Image or
/// an already-unbounded FloatImage alike (via FloatImage::from_value).
pub struct Screen {
    // GPU-backed dispatch state - see blur.rs's identical fields for the
    // full rationale.
    gpu_pipeline: RefCell<Option<ScreenGpuPipeline>>,
    pending: Rc<RefCell<Option<ScreenFingerprint>>>,
    last_gpu_result: Rc<RefCell<Option<CompletedScreenJob>>>,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            gpu_pipeline: RefCell::new(None),
            pending: Rc::new(RefCell::new(None)),
            last_gpu_result: Rc::new(RefCell::new(None)),
        }
    }

    /// Mirrors `Add::dispatch_gpu` in structure.
    fn dispatch_gpu(&self, gpu: Arc<GpuState>, fingerprint: ScreenFingerprint, foreground: FloatImage, background: FloatImage) {
        let width = foreground.width;
        let height = foreground.height;
        let len = foreground.pixels.len();
        let byte_len = (len * std::mem::size_of::<f32>()) as u64;

        {
            let mut pipeline_slot = self.gpu_pipeline.borrow_mut();
            if pipeline_slot.is_none() {
                *pipeline_slot = Some(build_screen_pipeline(&gpu));
            }
        }

        let foreground_buffer = gpu.upload("screen foreground", &foreground.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let background_buffer = gpu.upload("screen background", &background.pixels, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let output_buffer = gpu.create_buffer(
            "screen output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let params: [u32; 4] = [width, height, 0, 0];
        let params_buffer = gpu.upload("screen params", &params, wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);

        let readback_buffer = gpu.create_buffer(
            "screen readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        {
            let pipeline_slot = self.gpu_pipeline.borrow();
            let pipeline = pipeline_slot.as_ref().expect("just built above");

            let bind_group = gpu.create_bind_group("screen bind group", &pipeline.bind_group_layout, &[
                wgpu::BindGroupEntry { binding: 0, resource: foreground_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: background_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: output_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_buffer.as_entire_binding() },
            ]);

            let workgroups_x = width.div_ceil(8);
            let workgroups_y = height.div_ceil(8);
            gpu.dispatch("screen dispatch", &pipeline.pipeline, &bind_group, (workgroups_x, workgroups_y, 1));
        }

        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let pixels = gpu.read_buffer_blocking(&readback_buffer, len);
            *self.last_gpu_result.borrow_mut() = Some(CompletedScreenJob { fingerprint, pixels, width, height });
            *self.pending.borrow_mut() = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            *self.pending.borrow_mut() = Some(fingerprint.clone());
            let pending = self.pending.clone();
            let last_gpu_result = self.last_gpu_result.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pixels = gpu.read_buffer_async(&readback_buffer, len).await;
                *last_gpu_result.borrow_mut() = Some(CompletedScreenJob {
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

    /// Screen two RGBA pixel buffers channel by channel - NOT clamped.
    pub fn screen_pixels(a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut output = vec![0f32; a.len()];

        for ((source_a, source_b), target) in a
            .chunks_exact(4)
            .zip(b.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            for channel in 0..4 {
                let inv_a = 1.0 - source_a[channel];
                let inv_b = 1.0 - source_b[channel];
                target[channel] = 1.0 - inv_a * inv_b;
            }
        }

        output
    }

    /// The screened value of a single pixel, computed directly from `a`/`b`
    /// - identical math to `screen_pixels`'s own loop body for that index.
    /// Used by `execute()`'s bbox-restricted path (Phase 3 of
    /// BBOX_CONVENTIONS.md).
    fn screen_single_pixel(a: &[f32], b: &[f32], x: u32, y: u32, width: u32) -> [f32; 4] {
        let idx = ((y * width + x) * 4) as usize;
        let mut out = [0f32; 4];
        for c in 0..4 {
            let inv_a = 1.0 - a[idx + c];
            let inv_b = 1.0 - b[idx + c];
            out[c] = 1.0 - inv_a * inv_b;
        }
        out
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Screen {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "screen",
            menu: "COMPOSE",
            label: "SCREEN",
            action: None,
            ui_action: None,
            create_node: Some("screen"),
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
            display_name: "Screen",
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

    fn set_parameter(&mut self, name: &str, _value: Value) -> Result<(), OperationError> {
        Err(OperationError::UnknownParameter(name.to_string()))
    }

    // See blur.rs's identical override for the full rationale.
    fn is_live(&self) -> bool {
        self.pending.borrow().is_some()
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(first) = find_input(inputs, Input::Foreground) else {
            return Err(OperationError::InvalidInputType("Screen requires first input".into()));
        };

        let Some(second) = find_input(inputs, Input::Background) else {
            return Err(OperationError::InvalidInputType("Screen requires second input".into()));
        };

        let first_image = FloatImage::from_value(first, ctx)?;
        let second_image = FloatImage::from_value(second, ctx)?;

        if first_image.width != second_image.width || first_image.height != second_image.height {
            return Err(OperationError::InvalidInputType(
                "Screen inputs must have matching dimensions".into()
            ));
        }

        let mask = find_input(inputs, Input::Mask)
            .map(|v| crate::graphics::resolve_pixels(v, ctx))
            .transpose()?;

        // Phase 3 of BBOX_CONVENTIONS.md: with a MASK wired, apply_mask
        // below blends every pixel outside the relevant region straight
        // back to Foreground (this operation's own "original", per
        // apply_mask's first argument below) anyway - so restrict the
        // actual screen compute to the intersection of MASK's own
        // reported box and this operation's own natural box.
        //
        // Unlike BLUR/CHROMA KEY/SHUFFLE, that natural box is NOT simply
        // one input's own box: SCREEN is not zero-preserving on either
        // input alone - screening black with a real Background produces
        // that real Background unchanged (see screening_with_black_is_
        // identity), so a pixel where Foreground is default but
        // Background is real is still genuinely non-default output. The
        // natural box is therefore the UNION of Foreground's and
        // Background's own reported boxes - the region where EITHER
        // input could contribute real content.
        if mask.is_some() {
            let mask = mask.as_ref();
            let foreground_box = find_bbox(&ctx.input_bboxes, Input::Foreground)
                .unwrap_or_else(|| Rect::full(first_image.width, first_image.height));
            let background_box = find_bbox(&ctx.input_bboxes, Input::Background)
                .unwrap_or_else(|| Rect::full(first_image.width, first_image.height));
            let natural_box = foreground_box.union(&background_box);
            let mask_box = find_bbox(&ctx.input_bboxes, Input::Mask)
                .unwrap_or_else(|| Rect::full(first_image.width, first_image.height));
            let work_area = natural_box.intersect(&mask_box);

            let width = first_image.width;
            let a = &first_image.pixels;
            let b = &second_image.pixels;

            let screened = crate::graphics::compute_within_bbox(width, first_image.height, work_area, a, |x, y| {
                Self::screen_single_pixel(a, b, x, y, width)
            });
            let screened = crate::graphics::apply_mask(&first_image.pixels, screened, mask, first_image.width, first_image.height)?;

            return Ok(vec![Value::FloatImage(Arc::new(FloatImage { pixels: screened, width: first_image.width, height: first_image.height }))]);
        }

        // Unmasked path: try GPU first when available.
        if let Some(gpu) = ctx.gpu.clone() {
            let fingerprint = ScreenFingerprint { foreground: first.clone(), background: second.clone() };

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

        let screened = Self::screen_pixels(&first_image.pixels, &second_image.pixels);

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels: screened,
            width: first_image.width,
            height: first_image.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Screen::new())
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
    fn screening_with_black_is_identity() {
        // All 4 channels are screened uniformly (matching Multiply's own
        // convention), so "black" here means every channel including
        // alpha is 0 - a channel left at 255 would screen as that
        // channel's own "white" case instead.
        let black = float_pixels(vec![0, 0, 0, 0]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Screen::screen_pixels(&black, &FloatImage::from_image(&color).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);

        assert_eq!(out.pixels, color.pixels);
    }

    #[test]
    fn screening_with_white_is_white() {
        let white = float_pixels(vec![255, 255, 255, 255]);
        let color = image(vec![10, 20, 30, 200], 1, 1);

        let out = Screen::screen_pixels(&white, &FloatImage::from_image(&color).pixels);
        let out = FloatImage { pixels: out, width: 1, height: 1 }.to_image_clamped(0.0, 1.0);

        assert_eq!(out.pixels, vec![255, 255, 255, 255]);
    }

    #[test]
    fn screen_is_invert_multiply_invert() {
        use crate::operations::compose::Multiply;

        let a = float_pixels(vec![80, 150, 30, 255]);
        let b = float_pixels(vec![200, 10, 90, 255]);

        let direct = Screen::screen_pixels(&a, &b);

        use crate::operations::transform::Invert;

        let inv_a = Invert::invert_pixels(&a);
        let inv_b = Invert::invert_pixels(&b);
        let multiplied = Multiply::multiply_pixels(&inv_a, &inv_b);
        let via_identity = Invert::invert_pixels(&multiplied);

        for (x, y) in direct.iter().zip(via_identity.iter()) {
            assert!((x - y).abs() < 0.001);
        }
    }

    #[test]
    fn screen_combines_two_wired_inputs() {
        let screen = Screen::new();

        let fg = Value::Image(image(vec![0, 0, 0, 255], 1, 1));
        let bg = Value::Image(image(vec![10, 20, 30, 255], 1, 1));

        let values = screen
            .execute(&context(1, 1), &[(Input::Foreground, fg), (Input::Background, bg)])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), vec![10, 20, 30, 255]);
    }

    #[test]
    fn a_zero_alpha_mask_passes_through_foreground_unscreened() {
        let screen = Screen::new();
        let fg = image(vec![10, 20, 30, 255], 1, 1);
        let bg = image(vec![255, 255, 255, 255], 1, 1);
        let mask = image(vec![0, 0, 0, 0], 1, 1);

        let values = screen
            .execute(&context(1, 1), &[
                (Input::Foreground, Value::Image(fg.clone())),
                (Input::Background, Value::Image(bg)),
                (Input::Mask, Value::Image(mask)),
            ])
            .unwrap();

        assert_eq!(as_u8_pixels(&values[0]), fg.pixels);
    }

    #[test]
    fn consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one() {
        let screen = Screen::new();

        let fg = image((0..6).flat_map(|n| [n * 10, n * 10 + 1, n * 10 + 2, 255]).collect(), 6, 1);
        let bg = image((0..6).flat_map(|n| [255 - n * 10, 100, 50, 255]).collect(), 6, 1);
        let mask = image(
            vec![
                0, 0, 0, 0,   0, 0, 0, 0,
                0, 0, 0, 255, 0, 0, 0, 255,
                0, 0, 0, 0,   0, 0, 0, 0,
            ],
            6, 1,
        );

        let inputs = [
            (Input::Foreground, Value::Image(fg)),
            (Input::Background, Value::Image(bg)),
            (Input::Mask, Value::Image(mask)),
        ];

        let ctx_with_real_box = Context {
            input_bboxes: vec![
                (Input::Foreground, Rect::full(6, 1)),
                (Input::Background, Rect::full(6, 1)),
                (Input::Mask, Rect { x0: 2, y0: 0, x1: 4, y1: 1 }),
            ],
            ..context(6, 1)
        };
        let ctx_full_frame = context(6, 1);

        let restricted = screen.execute(&ctx_with_real_box, &inputs).unwrap();
        let unrestricted = screen.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn consume_equivalence_requires_the_union_not_the_intersection_of_foreground_and_background_boxes() {
        // The load-bearing test for SCREEN specifically: unlike BLUR's
        // (evaluator-caught) mistake of using the wrong single-input box,
        // SCREEN's natural box must be the UNION of Foreground's and
        // Background's own boxes, not the intersection (and not just one
        // of them). Foreground is entirely default (black/transparent)
        // everywhere; Background carries the only real content, confined
        // to [3,7). If the code wrongly used only Foreground's box (empty
        // here) or the intersection (also empty, since Foreground's box
        // is empty), work_area would incorrectly become empty and the
        // real Background content would never get screened in - the
        // restricted result would wrongly show Foreground (black)
        // everywhere instead of Background's real colour.
        let screen = Screen::new();

        let fg = image(vec![0u8; 10 * 4], 10, 1);
        let mut bg_pixels = vec![0u8; 10 * 4];
        for x in 3..7 {
            bg_pixels[x * 4..x * 4 + 4].copy_from_slice(&[200, 150, 100, 255]);
        }
        let bg = image(bg_pixels, 10, 1);
        let mask = image(vec![255; 10 * 4], 10, 1);

        let inputs = [
            (Input::Foreground, Value::Image(fg)),
            (Input::Background, Value::Image(bg)),
            (Input::Mask, Value::Image(mask)),
        ];

        // Foreground reports an empty box (genuinely no real content
        // anywhere); Background reports its own real [3,7) box.
        let ctx_with_real_boxes = Context {
            input_bboxes: vec![
                (Input::Foreground, Rect::empty()),
                (Input::Background, Rect { x0: 3, y0: 0, x1: 7, y1: 1 }),
                (Input::Mask, Rect::full(10, 1)),
            ],
            ..context(10, 1)
        };
        let ctx_full_frame = context(10, 1);

        let restricted = screen.execute(&ctx_with_real_boxes, &inputs).unwrap();
        let unrestricted = screen.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));

        // Directly pin down that Background's real content actually got
        // screened in, not silently skipped.
        let restricted_pixels = as_u8_pixels(&restricted[0]);
        assert_eq!(&restricted_pixels[4 * 4..4 * 4 + 4], &[200, 150, 100, 255], "Background's real content at x=4 must be screened in, not left as Foreground's black");
    }

    #[test]
    fn a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one() {
        use crate::graphics::mask::{reset_pixels_computed, take_pixels_computed};

        let screen = Screen::new();

        let fg = image((0..16).flat_map(|n| [n, n, n, 255]).collect(), 4, 4);
        let bg = image((0..16).flat_map(|n| [255 - n, n, 128, 255]).collect(), 4, 4);
        let mask = image(vec![255; 4 * 4 * 4], 4, 4);

        let inputs = [
            (Input::Foreground, Value::Image(fg)),
            (Input::Background, Value::Image(bg)),
            (Input::Mask, Value::Image(mask)),
        ];

        let small_box_ctx = Context {
            input_bboxes: vec![
                (Input::Foreground, Rect::full(4, 4)),
                (Input::Background, Rect::full(4, 4)),
                (Input::Mask, Rect { x0: 1, y0: 1, x1: 2, y1: 2 }),
            ],
            ..context(4, 4)
        };
        reset_pixels_computed();
        screen.execute(&small_box_ctx, &inputs).unwrap();
        let small_box_pixels = take_pixels_computed().expect("SCREEN with a wired MASK must record a pixel count");

        let full_frame_ctx = context(4, 4);
        reset_pixels_computed();
        screen.execute(&full_frame_ctx, &inputs).unwrap();
        let full_frame_pixels = take_pixels_computed().expect("SCREEN with a wired MASK must record a pixel count");

        assert_eq!(small_box_pixels, 1);
        assert_eq!(full_frame_pixels, 16);
        assert!(small_box_pixels < full_frame_pixels);
    }

    #[test]
    fn checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor, RenderExecutor};
        use crate::operations::generators::Checkerboard;
        use crate::graphics::Color;
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::{Move, Resize};

        let mut graph = Graph::new(4, 4);

        let mut fg_source = ImageSource::new();
        fg_source.set_image(image((0..16).flat_map(|n| [n * 15, 0, 0, 255]).collect(), 4, 4));
        let fg_id = graph.add_node(Box::new(fg_source));

        let mut bg_source = ImageSource::new();
        bg_source.set_image(image((0..16).flat_map(|n| [0, n * 15, 0, 255]).collect(), 4, 4));
        let bg_id = graph.add_node(Box::new(bg_source));

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

        let screen_id = graph.add_node(Box::new(Screen::new()));
        graph.connect(screen_id, Input::Foreground, fg_id).unwrap();
        graph.connect(screen_id, Input::Background, bg_id).unwrap();
        graph.connect(screen_id, Input::Mask, move_id).unwrap();

        graph.validate().expect("the wired pipeline is valid");

        let ctx = context(4, 4);

        let on_values = RenderExecutor::new().execute(&graph, screen_id, &ctx).unwrap();
        let on_pixels = as_u8_pixels(&on_values[0]);

        let fg_value = PreviewExecutor::default().execute(&graph, fg_id, &ctx).unwrap().into_iter().next().unwrap();
        let bg_value = PreviewExecutor::default().execute(&graph, bg_id, &ctx).unwrap().into_iter().next().unwrap();
        let mask_value = PreviewExecutor::default().execute(&graph, move_id, &ctx).unwrap().into_iter().next().unwrap();

        let screen_off = Screen::new();
        let off_values = screen_off.execute(&ctx, &[
            (Input::Foreground, fg_value),
            (Input::Background, bg_value),
            (Input::Mask, mask_value),
        ]).unwrap();
        let off_pixels = as_u8_pixels(&off_values[0]);

        assert_eq!(on_pixels, off_pixels, "bbox consumption on vs off must produce pixel-identical output");
    }

    // --- WebGPU Phase 1.3 (SPECwebgpuoperations.md) ---

    #[test]
    fn is_live_is_true_only_while_a_gpu_dispatch_is_pending() {
        let screen = Screen::new();
        assert!(!screen.is_live());

        *screen.pending.borrow_mut() = Some(ScreenFingerprint { foreground: Value::Number(0.0), background: Value::Number(0.0) });
        assert!(screen.is_live());

        *screen.pending.borrow_mut() = None;
        assert!(!screen.is_live());
    }

    #[test]
    fn gpu_screen_matches_cpu_within_tolerance_once_warmed_up() {
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

        let cpu_screen = Screen::new();
        let cpu_values = cpu_screen
            .execute(&context(width, height), &[(Input::Foreground, Value::Image(fg.clone())), (Input::Background, Value::Image(bg.clone()))])
            .unwrap();
        let Value::FloatImage(cpu_result) = &cpu_values[0] else { panic!("expected a float image") };

        let gpu_screen = Screen::new();
        let gpu_ctx = Context { gpu: Some(gpu), ..context(width, height) };
        let _ = gpu_screen.execute(&gpu_ctx, &[(Input::Foreground, Value::Image(fg.clone())), (Input::Background, Value::Image(bg.clone()))]).unwrap();
        let gpu_values = gpu_screen.execute(&gpu_ctx, &[(Input::Foreground, Value::Image(fg)), (Input::Background, Value::Image(bg))]).unwrap();
        let Value::FloatImage(gpu_result) = &gpu_values[0] else { panic!("expected a float image") };

        assert_eq!(cpu_result.pixels.len(), gpu_result.pixels.len());
        for (index, (cpu_px, gpu_px)) in cpu_result.pixels.iter().zip(gpu_result.pixels.iter()).enumerate() {
            assert!((cpu_px - gpu_px).abs() < 1e-4, "channel {}: cpu={}, gpu={}", index, cpu_px, gpu_px);
        }
    }
}
