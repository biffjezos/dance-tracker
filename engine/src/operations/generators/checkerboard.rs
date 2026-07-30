// src/operations/generator/checkerboard.rs

use std::any::Any;
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
    Image,
    ImageFormat,
};
use crate::graphics::Color;

pub struct Checkerboard {
    pub size: f64,
    pub color_a: Color,
    pub color_b: Color,
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
        }
    }

    fn color_to_rgba(color: Color) -> [u8; 4] {
        [
            (color.r.clamp(0.0, 1.0) * 255.0) as u8,
            (color.g.clamp(0.0, 1.0) * 255.0) as u8,
            (color.b.clamp(0.0, 1.0) * 255.0) as u8,
            (color.a.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }

    pub fn generate(&self, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        let a = Self::color_to_rgba(self.color_a);
        let b = Self::color_to_rgba(self.color_b);

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
                kind: ParameterKind::Number { step: 1.0 },
            },
            ParameterDescriptor {
                name: "color_a",
                kind: ParameterKind::Color,
            },
            ParameterDescriptor {
                name: "color_b",
                kind: ParameterKind::Color,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "size" => Some(Value::Number(self.size)),
            "color_a" => Some(Value::Color(self.color_a)),
            "color_b" => Some(Value::Color(self.color_b)),
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

            "color_a" => {
                if let Value::Color(color) = value {
                    self.color_a = color;
                    Ok(())
                } else {
                    Err(OperationError::InvalidParameterType(name.to_string()))
                }
            }

            "color_b" => {
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

    fn execute(
        &self,
        ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        Ok(vec![
            Value::Image(Arc::new(Image {
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
}