// src/operations/transform/rgb_to_hsv.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    input::find_input,
    Operation,
    OperationDescriptor,
    metadata::{OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind},
    Value,
};
use crate::graphics::{Color, FloatImage};

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
}

impl RgbToHsv {
    pub fn new() -> Self {
        Self { format: ColorFormat::default() }
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
            inputs: vec![Input::Source],
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

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(crate::graphics::U8Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

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
}
