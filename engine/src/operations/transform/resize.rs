// src/operations/transform/resize.rs
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
use crate::graphics::{Frame, Image};

/// Resampling algorithm for RESIZE. Only NEAREST_NEIGHBOR exists today - the
/// enum and the single-entry options list both exist already so adding
/// BILINEAR later is just a new match arm and a new string in the list, not
/// a parameter shape change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResizeAlgorithm {
    NearestNeighbor,
}

pub const RESIZE_ALGORITHMS: &[&str] = &["NEAREST NEIGHBOR"];

impl Default for ResizeAlgorithm {
    fn default() -> Self {
        ResizeAlgorithm::NearestNeighbor
    }
}

impl ResizeAlgorithm {
    pub fn to_str(&self) -> &'static str {
        match self {
            ResizeAlgorithm::NearestNeighbor => "NEAREST NEIGHBOR",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "NEAREST NEIGHBOR" => Some(ResizeAlgorithm::NearestNeighbor),
            _ => None,
        }
    }
}

/// Scales a node's own content around its frame's center, keeping the
/// frame's width/height unchanged - a digital zoom, not a canvas resize.
/// Shrinking (scale < 100) leaves the uncovered edges fully transparent
/// (visible via the transparency checker); zooming in (scale > 100) always
/// samples from inside the original frame, so it crops rather than pads.
pub struct Resize {
    pub scale_x: f64,
    pub scale_y: f64,
    pub algorithm: ResizeAlgorithm,
}

impl Resize {
    pub fn new() -> Self {
        Self {
            scale_x: 100.0,
            scale_y: 100.0,
            algorithm: ResizeAlgorithm::default(),
        }
    }

    /// Nearest-neighbor resample of an RGBA buffer, scaled `scale_x`/`scale_y`
    /// percent around the frame's own center. Pixels whose inverse-mapped
    /// source coordinate falls outside the original frame come out as
    /// transparent black (all-zero), not clamped to the edge.
    pub fn resize_pixels(pixels: &[u8], width: u32, height: u32, scale_x: f64, scale_y: f64) -> Vec<u8> {
        let mut output = vec![0u8; pixels.len()];

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let inv_x = 100.0 / scale_x;
        let inv_y = 100.0 / scale_y;

        for y in 0..height {
            for x in 0..width {
                let src_x = cx + (x as f64 + 0.5 - cx) * inv_x;
                let src_y = cy + (y as f64 + 0.5 - cy) * inv_y;

                if src_x < 0.0 || src_y < 0.0 || src_x >= width as f64 || src_y >= height as f64 {
                    continue;
                }

                let sx = src_x as u32;
                let sy = src_y as u32;

                let dest_index = ((y * width + x) * 4) as usize;
                let src_index = ((sy * width + sx) * 4) as usize;
                output[dest_index..dest_index + 4].copy_from_slice(&pixels[src_index..src_index + 4]);
            }
        }

        output
    }
}

impl Default for Resize {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Resize {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "resize",
            menu: "TRANSFORM",
            label: "RESIZE",
            action: None,
            ui_action: None,
            create_node: Some("resize"),
            submenu: Some("KINETIC"),
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
            display_name: "Resize",
            category: OperationCategory::Color,
            // Deliberately no Input::Mask: a MASK input needs to be the
            // same dimensions as both the "identity" pass-through and the
            // processed result it blends against (see graphics::apply_mask),
            // but Resize's own output is a *different* size than its input
            // at any scale != 100% - there's no single pixel-for-pixel
            // identity to blend a mask against here.
            inputs: vec![Input::Source],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "SCALE_X",
                kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: Some(1000.0) },
                group: None,
            },
            ParameterDescriptor {
                name: "SCALE_Y",
                kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: Some(1000.0) },
                group: None,
            },
            ParameterDescriptor {
                name: "ALGORITHM",
                kind: ParameterKind::Enum(RESIZE_ALGORITHMS),
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "SCALE_X" => Some(Value::Number(self.scale_x)),
            "SCALE_Y" => Some(Value::Number(self.scale_y)),
            "ALGORITHM" => Some(Value::Text(self.algorithm.to_str().to_string())),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("SCALE_X", Value::Number(v)) => {
                if v < 1.0 || v > 1000.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.scale_x = v;
                Ok(())
            }
            ("SCALE_Y", Value::Number(v)) => {
                if v < 1.0 || v > 1000.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.scale_y = v;
                Ok(())
            }
            ("ALGORITHM", Value::Text(s)) => {
                self.algorithm = ResizeAlgorithm::from_str(&s)
                    .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![Value::Image(Image::missing(ctx.meta.width, ctx.meta.height))]);
        };

        match value {
            Value::Frame(frame) => Ok(vec![Value::Frame(Arc::new(Frame {
                pixels: Self::resize_pixels(&frame.pixels, frame.width, frame.height, self.scale_x, self.scale_y),
                width: frame.width,
                height: frame.height,
                timestamp: frame.timestamp,
            }))]),

            Value::Image(image) => Ok(vec![Value::Image(Arc::new(Image {
                pixels: Self::resize_pixels(&image.pixels, image.width, image.height, self.scale_x, self.scale_y),
                width: image.width,
                height: image.height,
                format: image.format,
            }))]),

            Value::Video(video) => {
                let image = video.frame_at(ctx.meta.time)?;
                Ok(vec![Value::Image(Arc::new(Image {
                    pixels: Self::resize_pixels(&image.pixels, image.width, image.height, self.scale_x, self.scale_y),
                    width: image.width,
                    height: image.height,
                    format: image.format,
                }))])
            }

            other => Err(OperationError::InvalidInputType(format!(
                "Resize cannot process {:?}",
                other
            ))),
        }
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Resize::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::graph::Graph;
    use crate::compositor::executors::{Execute, RenderExecutor};

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn scale_100_is_identity() {
        let pixels: Vec<u8> = (0..16).map(|n| (n * 16) as u8).collect(); // 2x2 RGBA
        let out = Resize::resize_pixels(&pixels, 2, 2, 100.0, 100.0);
        assert_eq!(out, pixels);
    }

    #[test]
    fn shrinking_pads_edges_with_transparency() {
        // A solid opaque 4x4 image shrunk to 50% leaves a transparent
        // border around a smaller opaque center.
        let pixels: Vec<u8> = (0..4 * 4).flat_map(|_| [10, 20, 30, 255]).collect();
        let out = Resize::resize_pixels(&pixels, 4, 4, 50.0, 50.0);

        // Corner pixel maps outside the original frame once shrunk - transparent.
        assert_eq!(&out[0..4], &[0, 0, 0, 0]);

        // Center pixels still sample real (opaque) content.
        let center_index = ((2 * 4 + 2) * 4) as usize;
        assert_eq!(&out[center_index..center_index + 4], &[10, 20, 30, 255]);
    }

    #[test]
    fn zooming_in_never_pads_with_transparency() {
        let pixels = (0..(4 * 4))
            .flat_map(|_| [10u8, 20, 30, 255])
            .collect::<Vec<u8>>();
        let out = Resize::resize_pixels(&pixels, 4, 4, 200.0, 200.0);

        // Every pixel came from somewhere inside the original (opaque) frame.
        for chunk in out.chunks_exact(4) {
            assert_eq!(chunk[3], 255);
        }
    }

    #[test]
    fn set_parameter_rejects_an_out_of_range_scale() {
        let mut resize = Resize::new();
        let err = resize.set_parameter("SCALE_X", Value::Number(0.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn set_parameter_rejects_an_unknown_algorithm() {
        let mut resize = Resize::new();
        let err = resize.set_parameter("ALGORITHM", Value::Text("BILINEAR".into())).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn resize_in_graph_is_valid() {
        let mut graph = Graph::new(4, 4);
        let resize_id = graph.add_node(Box::new(Resize::new()));
        graph.validate().expect("unwired resize is valid");
        RenderExecutor::new()
            .execute(&graph, resize_id, &context(4, 4))
            .expect("unwired resize renders");
    }
}
