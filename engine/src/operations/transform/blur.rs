// src/operations/filter/blur.rs
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
use crate::graphics::{Frame, Image, ImageFormat};

/// A simple separable box blur.
///
/// radius_px = 0 means “no blur” (identity).
/// radius_px > 0 applies a box kernel of width (2 * radius + 1).
pub struct Blur {
    pub radius_px: u32,
}

impl Blur {
    pub fn new() -> Self {
        Self { radius_px: 0 }
    }

    /// Apply a separable box blur to an RGBA buffer.
    ///
    /// This is a very basic implementation:
    /// - Horizontal pass, then vertical pass.
    /// - Clamps at edges (no special border modes).
    /// - Operates in u8, no gamma/linear handling.
    pub fn blur_pixels(&self, pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
        if self.radius_px == 0 {
            return pixels.to_vec();
        }

        let w = width as usize;
        let h = height as usize;
        let r = self.radius_px as usize;

        let mut tmp = vec![0u8; pixels.len()];
        let mut out = vec![0u8; pixels.len()];

        // Horizontal pass
        for y in 0..h {
            let row_start = y * w * 4;
            for x in 0..w {
                let mut sum = [0u32; 4];
                let mut count = 0u32;

                let x_start = x.saturating_sub(r);
                let x_end = (x + r).min(w - 1);

                for xi in x_start..=x_end {
                    let idx = (row_start + xi * 4) as usize;
                    sum[0] += pixels[idx] as u32;
                    sum[1] += pixels[idx + 1] as u32;
                    sum[2] += pixels[idx + 2] as u32;
                    sum[3] += pixels[idx + 3] as u32;
                    count += 1;
                }

                let inv = 1.0 / count as f32;
                let out_idx = row_start + x * 4;
                tmp[out_idx] = (sum[0] as f32 * inv) as u8;
                tmp[out_idx + 1] = (sum[1] as f32 * inv) as u8;
                tmp[out_idx + 2] = (sum[2] as f32 * inv) as u8;
                tmp[out_idx + 3] = (sum[3] as f32 * inv) as u8;
            }
        }

        // Vertical pass
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0u32; 4];
                let mut count = 0u32;

                let y_start = y.saturating_sub(r);
                let y_end = (y + r).min(h - 1);

                for yi in y_start..=y_end {
                    let idx = (yi * w * 4 + x * 4) as usize;
                    sum[0] += tmp[idx] as u32;
                    sum[1] += tmp[idx + 1] as u32;
                    sum[2] += tmp[idx + 2] as u32;
                    sum[3] += tmp[idx + 3] as u32;
                    count += 1;
                }

                let inv = 1.0 / count as f32;
                let out_idx = y * w * 4 + x * 4;
                out[out_idx] = (sum[0] as f32 * inv) as u8;
                out[out_idx + 1] = (sum[1] as f32 * inv) as u8;
                out[out_idx + 2] = (sum[2] as f32 * inv) as u8;
                out[out_idx + 3] = (sum[3] as f32 * inv) as u8;
            }
        }

        out
    }
}

impl Default for Blur {
    fn default() -> Self {
        Self::new()
    }
}

fn black_image(width: u32, height: u32) -> Arc<Image> {
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    Arc::new(Image {
        pixels,
        width,
        height,
        format: ImageFormat::Rgba8,
    })
}

impl Operation for Blur {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "blur",
            menu: "TRANSFORM",
            label: "BLUR",
            action: None,
            ui_action: None,
            create_node: Some("blur"),
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
            display_name: "Blur",
            category: OperationCategory::Color,
            inputs: vec![Input::Source],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![ParameterDescriptor {
            name: "radius_px",
            kind: ParameterKind::Number { step: 0.1, min: Some(0.0), max: Some(1000.0) },
        }]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "radius_px" => Some(Value::Number(self.radius_px as f64)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("radius_px", Value::Number(v)) => {
                if v < 0.0 || v > 1000.0 {
                    return Err(OperationError::InvalidParameterValue(
                        name.to_string(),
                        v.to_string(),
                    ));
                }
                self.radius_px = v as u32;
                Ok(())
            }
            _ => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(black_image(ctx.meta.width, ctx.meta.height))]);
        };

        match value {
            Value::Frame(frame) => {
                let blurred = self.blur_pixels(&frame.pixels, frame.width, frame.height);
                Ok(vec![Value::Frame(Arc::new(Frame {
                    pixels: blurred,
                    width: frame.width,
                    height: frame.height,
                    timestamp: frame.timestamp,
                }))])
            }

            Value::Image(image) => {
                let blurred = self.blur_pixels(&image.pixels, image.width, image.height);
                Ok(vec![Value::Image(Arc::new(Image {
                    pixels: blurred,
                    width: image.width,
                    height: image.height,
                    format: image.format,
                }))])
            }

            Value::Video(video) => {
                let image = video.frame_at(ctx.meta.time)?;
                let blurred = self.blur_pixels(&image.pixels, image.width, image.height);
                Ok(vec![Value::Image(Arc::new(Image {
                    pixels: blurred,
                    width: image.width,
                    height: image.height,
                    format: image.format,
                }))])
            }

            other => Err(OperationError::InvalidInputType(format!(
                "Blur cannot process {:?}",
                other
            ))),
        }
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Blur::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::graph::Graph;
    use crate::compositor::executors::{Execute, RenderExecutor};
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

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<Image> {
        Arc::new(Image {
            pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        })
    }

    #[test]
    fn zero_radius_is_identity() {
        let blur = Blur::new();
        let input = image(vec![10, 20, 30, 40, 50, 60, 70, 80], 2, 1);

        let values = blur
            .execute(&context(2, 1), &[(Input::Source, Value::Image(input.clone()))])
            .unwrap();

        match &values[0] {
            Value::Image(out) => assert_eq!(out.pixels, input.pixels),
            other => panic!("expected image, got {:?}", other),
        }
    }

    #[test]
    fn unconnected_blur_produces_black() {
        let blur = Blur::new();
        let values = blur.execute(&context(2, 1), &[]).unwrap();

        match &values[0] {
            Value::Image(out) => {
                assert_eq!(out.width, 2);
                assert_eq!(out.height, 1);
                assert_eq!(out.pixels, vec![0, 0, 0, 255, 0, 0, 0, 255]);
            }
            other => panic!("expected image, got {:?}", other),
        }
    }

    #[test]
    fn blur_in_graph_is_valid() {
        let mut graph = Graph::new(4, 4);
        let blur_id = graph.add_node(Box::new(Blur::new()));
        graph.validate().expect("unwired blur is valid");
        RenderExecutor
            .execute(&graph, blur_id, &context(4, 4))
            .expect("unwired blur renders");
    }
}