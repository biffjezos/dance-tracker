// src/operations/transform/shuffle.rs
use std::any::Any;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    Operation,
    OperationDescriptor,
    metadata::{ OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind },
    Value
};
use crate::graphics::{Image, ImageFormat};
use std::sync::Arc;

/// Channel selection for Shuffle operation
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShuffleChannel {
    Red,
    Green,
    Blue,
    Alpha,
    Off,
}

impl Default for ShuffleChannel {
    fn default() -> Self {
        ShuffleChannel::Red
    }
}

/// Shuffle operation - remaps RGBA channels
pub struct Shuffle {
    pub red: ShuffleChannel,
    pub green: ShuffleChannel,
    pub blue: ShuffleChannel,
    pub alpha: ShuffleChannel,
}

impl Shuffle {
    pub fn new() -> Self {
        Self {
            red: ShuffleChannel::Red,
            green: ShuffleChannel::Green,
            blue: ShuffleChannel::Blue,
            alpha: ShuffleChannel::Alpha,
        }
    }
    
    /// Get the value for a specific channel from input pixel
    fn get_channel_value(input: &Image, x: usize, y: usize, channel: ShuffleChannel) -> u8 {
        let index = (y * input.width as usize + x) * 4;
        if index + 3 >= input.pixels.len() {
            return 0;
        }
        match channel {
            ShuffleChannel::Red => input.pixels[index],
            ShuffleChannel::Green => input.pixels[index + 1],
            ShuffleChannel::Blue => input.pixels[index + 2],
            ShuffleChannel::Alpha => input.pixels[index + 3],
            ShuffleChannel::Off => 0,
        }
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
            buttons: &[],
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
            category: OperationCategory::Composite,
            input_count: 1,
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "red",
                kind: ParameterKind::Text,
            },
            ParameterDescriptor {
                name: "green",
                kind: ParameterKind::Text,
            },
            ParameterDescriptor {
                name: "blue",
                kind: ParameterKind::Text,
            },
            ParameterDescriptor {
                name: "alpha",
                kind: ParameterKind::Text,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "red" => Some(Value::Text(format!("{:?}", self.red))),
            "green" => Some(Value::Text(format!("{:?}", self.green))),
            "blue" => Some(Value::Text(format!("{:?}", self.blue))),
            "alpha" => Some(Value::Text(format!("{:?}", self.alpha))),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        if let Value::Text(s) = value {
            match name {
                "red" => {
                    self.red = parse_shuffle_channel(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "green" => {
                    self.green = parse_shuffle_channel(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "blue" => {
                    self.blue = parse_shuffle_channel(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "alpha" => {
                    self.alpha = parse_shuffle_channel(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                _ => return Err(OperationError::UnknownParameter(name.to_string())),
            }
            Ok(())
        } else {
            Err(OperationError::InvalidParameterType(name.to_string()))
        }
    }

    fn execute(&self, _ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        // Get the input image
        let input_value = inputs.first()
            .ok_or_else(|| OperationError::MissingInput("Shuffle requires an input".to_string()))?
            .1.clone();

        let input_image = match input_value {
            Value::Image(img) => img,
            _ => return Err(OperationError::InvalidInputType("Shuffle requires an Image input".to_string())),
        };

        // Create output image with same dimensions
        let width = input_image.width;
        let height = input_image.height;
        let mut output_pixels = vec![0u8; (width * height * 4) as usize];

        // Process each pixel
        for y in 0..height as usize {
            for x in 0..width as usize {
                let output_index = (y * width as usize + x) * 4;
                
                // Get values from source channels
                let r = Self::get_channel_value(&input_image, x, y, self.red);
                let g = Self::get_channel_value(&input_image, x, y, self.green);
                let b = Self::get_channel_value(&input_image, x, y, self.blue);
                let a = Self::get_channel_value(&input_image, x, y, self.alpha);

                output_pixels[output_index] = r;
                output_pixels[output_index + 1] = g;
                output_pixels[output_index + 2] = b;
                output_pixels[output_index + 3] = a;
            }
        }

        // Create output image
        let output_image = Arc::new(Image {
            pixels: output_pixels,
            width,
            height,
            format: ImageFormat::Rgba8,
        });

        Ok(vec![Value::Image(output_image)])
    }
}

/// Parse a string into ShuffleChannel
fn parse_shuffle_channel(s: &str) -> Option<ShuffleChannel> {
    match s.to_lowercase().as_str() {
        "red" => Some(ShuffleChannel::Red),
        "green" => Some(ShuffleChannel::Green),
        "blue" => Some(ShuffleChannel::Blue),
        "alpha" => Some(ShuffleChannel::Alpha),
        "off" => Some(ShuffleChannel::Off),
        _ => None,
    }
}

// Inventory registration for Shuffle
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Shuffle::new())
    }
}
