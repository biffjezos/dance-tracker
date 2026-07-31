// src/operations/key/chromakey.rs
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
use crate::graphics::{Color, Frame, Image};

/// What an unconnected SOURCE shows. A mask-producing node has nothing to
/// key "removal" against, so the busy missing()/transparency checker reads
/// as more confusing than helpful here - and is easy to mistake for real
/// content when eyedropping a colour straight off the canvas. SOLID (a
/// flat, obviously-fake colour) is the default; CHECKERBOARD is still
/// available for anyone who wants the usual missing-input convention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placeholder {
    Solid,
    Checkerboard,
}

pub const PLACEHOLDER_OPTIONS: &[&str] = &["SOLID", "CHECKERBOARD"];

impl Placeholder {
    pub fn to_str(&self) -> &'static str {
        match self {
            Placeholder::Solid => "SOLID",
            Placeholder::Checkerboard => "CHECKERBOARD",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "SOLID" => Some(Placeholder::Solid),
            "CHECKERBOARD" => Some(Placeholder::Checkerboard),
            _ => None,
        }
    }
}

/// Chroma-key: cuts a pixel's alpha to 0 wherever its colour is within
/// THRESHOLD of KEY_COLOR, leaving everything else untouched. Distance is
/// plain Euclidean over normalized RGB, divided by sqrt(3) so it lands in
/// 0..1 regardless of which two colours are furthest apart (black/white).
pub struct ChromaKey {
    pub key_color: Color,
    pub threshold: f64,
    pub placeholder: Placeholder,
    pub placeholder_color: Color,
}

impl ChromaKey {
    pub fn new() -> Self {
        Self {
            key_color: Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
            threshold: 0.3,
            placeholder: Placeholder::Solid,
            placeholder_color: Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 },
        }
    }

    /// Cut alpha to 0 for every pixel within `threshold` of `key_color`
    /// (both in normalized 0..1 RGB), leaving RGB and any already-lower
    /// alpha untouched otherwise.
    pub fn key_pixels(pixels: &[u8], key_color: Color, threshold: f64) -> Vec<u8> {
        let key_r = key_color.r as f64;
        let key_g = key_color.g as f64;
        let key_b = key_color.b as f64;

        let mut output = vec![0u8; pixels.len()];

        for (source, target) in pixels.chunks_exact(4).zip(output.chunks_exact_mut(4)) {
            let r = source[0] as f64 / 255.0;
            let g = source[1] as f64 / 255.0;
            let b = source[2] as f64 / 255.0;

            let distance = ((r - key_r).powi(2) + (g - key_g).powi(2) + (b - key_b).powi(2)).sqrt()
                / 3f64.sqrt();

            target[0] = source[0];
            target[1] = source[1];
            target[2] = source[2];
            target[3] = if distance <= threshold { 0 } else { source[3] };
        }

        output
    }
}

impl Default for ChromaKey {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for ChromaKey {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "chromakey",
            menu: "KEY",
            label: "CHROMA KEY",
            action: None,
            ui_action: None,
            create_node: Some("chromakey"),
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
            display_name: "Chroma Key",
            category: OperationCategory::Mask,
            inputs: vec![Input::Source],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "KEY_COLOR",
                kind: ParameterKind::Color,
                group: None,
            },
            ParameterDescriptor {
                name: "THRESHOLD",
                kind: ParameterKind::Number { step: 0.01, min: Some(0.0), max: Some(1.0) },
                group: None,
            },
            ParameterDescriptor {
                name: "PLACEHOLDER",
                kind: ParameterKind::Enum(PLACEHOLDER_OPTIONS),
                group: None,
            },
            ParameterDescriptor {
                name: "PLACEHOLDER_COLOR",
                kind: ParameterKind::Color,
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "KEY_COLOR" => Some(Value::Color(self.key_color)),
            "THRESHOLD" => Some(Value::Number(self.threshold)),
            "PLACEHOLDER" => Some(Value::Text(self.placeholder.to_str().to_string())),
            "PLACEHOLDER_COLOR" => Some(Value::Color(self.placeholder_color)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("KEY_COLOR", Value::Color(color)) => {
                self.key_color = color;
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
            ("PLACEHOLDER", Value::Text(s)) => {
                self.placeholder = Placeholder::from_str(&s)
                    .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                Ok(())
            }
            ("PLACEHOLDER_COLOR", Value::Color(color)) => {
                self.placeholder_color = color;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            let placeholder = match self.placeholder {
                Placeholder::Solid => Image::solid(self.placeholder_color, ctx.meta.width, ctx.meta.height),
                Placeholder::Checkerboard => Image::missing(ctx.meta.width, ctx.meta.height),
            };
            return Ok(vec![Value::Image(placeholder)]);
        };

        match value {
            Value::Frame(frame) => Ok(vec![Value::Frame(Arc::new(Frame {
                pixels: Self::key_pixels(&frame.pixels, self.key_color, self.threshold),
                width: frame.width,
                height: frame.height,
                timestamp: frame.timestamp,
            }))]),

            Value::Image(image) => Ok(vec![Value::Image(Arc::new(Image {
                pixels: Self::key_pixels(&image.pixels, self.key_color, self.threshold),
                width: image.width,
                height: image.height,
                format: image.format,
            }))]),

            Value::Video(video) => {
                let image = video.frame_at(ctx.meta.time)?;
                Ok(vec![Value::Image(Arc::new(Image {
                    pixels: Self::key_pixels(&image.pixels, self.key_color, self.threshold),
                    width: image.width,
                    height: image.height,
                    format: image.format,
                }))])
            }

            other => Err(OperationError::InvalidInputType(format!(
                "ChromaKey cannot process {:?}",
                other
            ))),
        }
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(ChromaKey::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::ImageFormat;

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<Image> {
        Arc::new(Image { pixels, width, height, format: ImageFormat::Rgba8 })
    }

    #[test]
    fn pure_green_is_keyed_out_at_default_settings() {
        let chromakey = ChromaKey::new();
        let green = image(vec![0, 255, 0, 255], 1, 1);

        let out = ChromaKey::key_pixels(&green.pixels, chromakey.key_color, chromakey.threshold);

        assert_eq!(out, vec![0, 255, 0, 0]);
    }

    #[test]
    fn a_far_colour_is_left_alone() {
        let chromakey = ChromaKey::new();
        let red = image(vec![255, 0, 0, 255], 1, 1);

        let out = ChromaKey::key_pixels(&red.pixels, chromakey.key_color, chromakey.threshold);

        assert_eq!(out, vec![255, 0, 0, 255]);
    }

    #[test]
    fn already_transparent_pixels_outside_the_key_stay_untouched() {
        let chromakey = ChromaKey::new();
        let translucent_red = image(vec![255, 0, 0, 100], 1, 1);

        let out = ChromaKey::key_pixels(&translucent_red.pixels, chromakey.key_color, chromakey.threshold);

        assert_eq!(out, vec![255, 0, 0, 100]);
    }

    #[test]
    fn threshold_zero_only_keys_the_exact_colour() {
        let key_color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
        let exact = image(vec![0, 255, 0, 255], 1, 1);
        let close = image(vec![10, 245, 10, 255], 1, 1);

        assert_eq!(ChromaKey::key_pixels(&exact.pixels, key_color, 0.0), vec![0, 255, 0, 0]);
        assert_eq!(ChromaKey::key_pixels(&close.pixels, key_color, 0.0), vec![10, 245, 10, 255]);
    }

    #[test]
    fn set_parameter_updates_key_color_and_threshold() {
        let mut chromakey = ChromaKey::new();
        chromakey.set_parameter("KEY_COLOR", Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 })).unwrap();
        chromakey.set_parameter("THRESHOLD", Value::Number(0.5)).unwrap();

        assert_eq!(chromakey.key_color, Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        assert_eq!(chromakey.threshold, 0.5);
    }

    #[test]
    fn set_parameter_rejects_out_of_range_threshold() {
        let mut chromakey = ChromaKey::new();
        let err = chromakey.set_parameter("THRESHOLD", Value::Number(1.5)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn an_unconnected_chromakey_is_solid_by_default_not_the_missing_checker() {
        let chromakey = ChromaKey::new();
        let values = chromakey.execute(&context(2, 1), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => {
                // Solid pink, not the missing()-style checker (which would
                // alternate magenta/black between pixels).
                assert_eq!(out.pixels, vec![255, 0, 255, 255, 255, 0, 255, 255]);
            }
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn checkerboard_placeholder_is_still_available_as_an_option() {
        let mut chromakey = ChromaKey::new();
        chromakey.set_parameter("PLACEHOLDER", Value::Text("CHECKERBOARD".into())).unwrap();

        let values = chromakey.execute(&context(2, 1), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, Image::missing(2, 1).pixels),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn set_parameter_rejects_an_unknown_placeholder() {
        let mut chromakey = ChromaKey::new();
        let err = chromakey.set_parameter("PLACEHOLDER", Value::Text("RAINBOW".into())).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }
}
