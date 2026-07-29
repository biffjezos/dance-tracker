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
/// Uses uppercase single-letter format for serialization: R, G, B, A, OFF
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShuffleChannel {
    R,
    G,
    B,
    A,
    Off,
}

impl Default for ShuffleChannel {
    fn default() -> Self {
        ShuffleChannel::R
    }
}

impl ShuffleChannel {
    /// Convert to serialization format (uppercase single letter or "OFF")
    pub fn to_str(&self) -> &'static str {
        match self {
            ShuffleChannel::R => "R",
            ShuffleChannel::G => "G",
            ShuffleChannel::B => "B",
            ShuffleChannel::A => "A",
            ShuffleChannel::Off => "OFF",
        }
    }

    /// Parse from serialization format
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "R" => Some(ShuffleChannel::R),
            "G" => Some(ShuffleChannel::G),
            "B" => Some(ShuffleChannel::B),
            "A" => Some(ShuffleChannel::A),
            "OFF" => Some(ShuffleChannel::Off),
            _ => None,
        }
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
            red: ShuffleChannel::R,
            green: ShuffleChannel::G,
            blue: ShuffleChannel::B,
            alpha: ShuffleChannel::A,
        }
    }
    
    /// Get the value for a specific channel from input pixel
    fn get_channel_value(input: &Image, x: usize, y: usize, channel: ShuffleChannel) -> u8 {
        let index = (y * input.width as usize + x) * 4;
        if index + 3 >= input.pixels.len() {
            return 0;
        }
        match channel {
            ShuffleChannel::R => input.pixels[index],
            ShuffleChannel::G => input.pixels[index + 1],
            ShuffleChannel::B => input.pixels[index + 2],
            ShuffleChannel::A => input.pixels[index + 3],
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
            category: OperationCategory::Color,
            input_count: 1,
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "red_channel",
                kind: ParameterKind::Text,
            },
            ParameterDescriptor {
                name: "green_channel",
                kind: ParameterKind::Text,
            },
            ParameterDescriptor {
                name: "blue_channel",
                kind: ParameterKind::Text,
            },
            ParameterDescriptor {
                name: "alpha_channel",
                kind: ParameterKind::Text,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "red_channel" => Some(Value::Text(self.red.to_str().to_string())),
            "green_channel" => Some(Value::Text(self.green.to_str().to_string())),
            "blue_channel" => Some(Value::Text(self.blue.to_str().to_string())),
            "alpha_channel" => Some(Value::Text(self.alpha.to_str().to_string())),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        if let Value::Text(s) = value {
            match name {
                "red_channel" => {
                    self.red = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "green_channel" => {
                    self.green = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "blue_channel" => {
                    self.blue = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "alpha_channel" => {
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

// Inventory registration for Shuffle
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Shuffle::new())
    }
}
