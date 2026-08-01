// src/operations/key/hue_key.rs
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
}

impl HueKey {
    pub fn new() -> Self {
        Self {
            hue_color: Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
            threshold: 0.1,
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
            inputs: vec![Input::Source, Input::Reference, Input::Mask],
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

        let keyed = Self::key_pixels(&source_image.pixels, &reference_image.pixels, target_hue, self.threshold);
        let keyed = crate::graphics::apply_mask(&source_image.pixels, keyed, mask.as_ref(), source_image.width, source_image.height)?;

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
