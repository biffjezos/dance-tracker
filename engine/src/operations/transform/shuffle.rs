// src/operations/transform/shuffle.rs
use std::any::Any;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    input::find_input,
    Operation,
    OperationDescriptor,
    metadata::{ OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind },
    Value
};
use crate::graphics::{Frame, Image};
use std::sync::Arc;

/// Channel selection for Shuffle operation.
/// OFF writes 0 into the target channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShuffleChannel {
    R,
    G,
    B,
    A,
    Off,
}

/// The complete set of values a channel selector may take - the single source
/// of truth for both parsing and what the UI is allowed to offer.
pub const SHUFFLE_CHANNELS: &[&str] = &[
    "RED",
    "GREEN",
    "BLUE",
    "ALPHA",
    "OFF",
];

impl Default for ShuffleChannel {
    fn default() -> Self {
        ShuffleChannel::R
    }
}

impl ShuffleChannel {
    pub fn to_str(&self) -> &'static str {
        match self {
            ShuffleChannel::R => "RED",
            ShuffleChannel::G => "GREEN",
            ShuffleChannel::B => "BLUE",
            ShuffleChannel::A => "ALPHA",
            ShuffleChannel::Off => "OFF",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "RED" | "R" => Some(ShuffleChannel::R),
            "GREEN" | "G" => Some(ShuffleChannel::G),
            "BLUE" | "B" => Some(ShuffleChannel::B),
            "ALPHA" | "A" => Some(ShuffleChannel::A),
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
    
    /// Get the value for one output channel out of a single RGBA pixel
    fn channel_value(pixel: &[u8], channel: ShuffleChannel) -> u8 {
        match channel {
            ShuffleChannel::R => pixel[0],
            ShuffleChannel::G => pixel[1],
            ShuffleChannel::B => pixel[2],
            ShuffleChannel::A => pixel[3],
            ShuffleChannel::Off => 0,
        }
    }

    /// Remap the channels of a packed RGBA buffer.
    /// Works on raw pixels so it applies to every pixel-bearing Value alike.
    pub fn shuffle_pixels(&self, pixels: &[u8]) -> Vec<u8> {
        let mut output = vec![0u8; pixels.len()];

        for (source, target) in pixels
            .chunks_exact(4)
            .zip(output.chunks_exact_mut(4))
        {
            target[0] = Self::channel_value(source, self.red);
            target[1] = Self::channel_value(source, self.green);
            target[2] = Self::channel_value(source, self.blue);
            target[3] = Self::channel_value(source, self.alpha);
        }

        output
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
            inputs: vec![Input::Source],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "RED_CHANNEL",
                kind: ParameterKind::Enum(SHUFFLE_CHANNELS),
                group: None,
            },
            ParameterDescriptor {
                name: "GREEN_CHANNEL",
                kind: ParameterKind::Enum(SHUFFLE_CHANNELS),
                group: None,
            },
            ParameterDescriptor {
                name: "BLUE_CHANNEL",
                kind: ParameterKind::Enum(SHUFFLE_CHANNELS),
                group: None,
            },
            ParameterDescriptor {
                name: "ALPHA_CHANNEL",
                kind: ParameterKind::Enum(SHUFFLE_CHANNELS),
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "RED_CHANNEL" => Some(Value::Text(self.red.to_str().to_string())),
            "GREEN_CHANNEL" => Some(Value::Text(self.green.to_str().to_string())),
            "BLUE_CHANNEL" => Some(Value::Text(self.blue.to_str().to_string())),
            "ALPHA_CHANNEL" => Some(Value::Text(self.alpha.to_str().to_string())),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        if let Value::Text(s) = value {
            match name {
                "RED_CHANNEL" => {
                    self.red = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "GREEN_CHANNEL" => {
                    self.green = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "BLUE_CHANNEL" => {
                    self.blue = ShuffleChannel::from_str(&s)
                        .ok_or_else(|| OperationError::InvalidParameterValue(name.to_string(), s))?;
                }
                "ALPHA_CHANNEL" => {
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

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        /*
        Nothing wired in yet: an unconnected SHUFFLE is a legal graph state,
        so it produces black at the current resolution rather than failing.
        */
        let Some(value) = find_input(inputs, Input::Source) else {
            return Ok(vec![
                Value::Image(Image::missing(ctx.meta.width, ctx.meta.height))
            ]);
        };

        /*
        Any pixel-bearing input is shuffled the same way; the output keeps the
        kind it came in as, so a live Frame stays a Frame down the chain.
        */
        match value {
            Value::Frame(frame) => {
                Ok(vec![Value::Frame(Arc::new(Frame {
                    pixels: self.shuffle_pixels(&frame.pixels),
                    width: frame.width,
                    height: frame.height,
                    timestamp: frame.timestamp,
                }))])
            }

            Value::Image(image) => {
                Ok(vec![Value::Image(Arc::new(Image {
                    pixels: self.shuffle_pixels(&image.pixels),
                    width: image.width,
                    height: image.height,
                    format: image.format,
                }))])
            }

            Value::Video(video) => {
                let image = video.frame_at(ctx.meta.time)?;

                Ok(vec![Value::Image(Arc::new(Image {
                    pixels: self.shuffle_pixels(&image.pixels),
                    width: image.width,
                    height: image.height,
                    format: image.format,
                }))])
            }

            other => Err(OperationError::InvalidInputType(
                format!("Shuffle cannot read channels from {:?}", other)
            )),
        }
    }
}

// Inventory registration for Shuffle
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Shuffle::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::graph::Graph;
    use crate::compositor::executors::{Execute, RenderExecutor};
    use crate::graphics::ImageFormat;
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
    fn an_unconnected_shuffle_produces_the_missing_placeholder_at_the_current_resolution() {
        let shuffle = Shuffle::new();

        let values = shuffle
            .execute(&context(2, 1), &[])
            .expect("an unwired shuffle is a legal graph state");

        match &values[0] {
            Value::Image(output) => {
                assert_eq!(output.width, 2);
                assert_eq!(output.height, 1);
                // Both pixels fall in the same 16px checker tile at this
                // tiny size, so they're the placeholder's magenta, not black.
                assert_eq!(output.pixels, vec![255, 0, 255, 255, 255, 0, 255, 255]);
            }
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn channels_are_taken_from_the_selected_source_channel() {
        let mut shuffle = Shuffle::new();

        shuffle.set_parameter("RED_CHANNEL", Value::Text("GREEN".into())).unwrap();
        shuffle.set_parameter("GREEN_CHANNEL", Value::Text("BLUE".into())).unwrap();
        shuffle.set_parameter("BLUE_CHANNEL", Value::Text("RED".into())).unwrap();
        shuffle.set_parameter("ALPHA_CHANNEL", Value::Text("OFF".into())).unwrap();

        let input = Value::Image(image(vec![10, 20, 30, 40], 1, 1));

        let values = shuffle
            .execute(&context(1, 1), &[(Input::Source, input)])
            .expect("shuffle should accept an image");

        match &values[0] {
            Value::Image(output) => assert_eq!(output.pixels, vec![20, 30, 10, 0]),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn off_zeroes_a_channel_without_touching_the_others() {
        let mut shuffle = Shuffle::new();
        shuffle.set_parameter("GREEN_CHANNEL", Value::Text("OFF".into())).unwrap();

        let input = Value::Image(image(vec![1, 2, 3, 4], 1, 1));

        let values = shuffle
            .execute(&context(1, 1), &[(Input::Source, input)])
            .expect("shuffle should accept an image");

        match &values[0] {
            Value::Image(output) => assert_eq!(output.pixels, vec![1, 0, 3, 4]),
            other => panic!("expected an image, got {:?}", other),
        }
    }

    #[test]
    fn a_live_frame_input_stays_a_frame() {
        let shuffle = Shuffle::new();

        let input = Value::Frame(Arc::new(Frame {
            pixels: vec![1, 2, 3, 4],
            width: 1,
            height: 1,
            timestamp: 12.5,
        }));

        let values = shuffle
            .execute(&context(1, 1), &[(Input::Source, input)])
            .expect("shuffle should accept a live frame");

        match &values[0] {
            Value::Frame(output) => {
                assert_eq!(output.pixels, vec![1, 2, 3, 4]);
                assert_eq!(output.timestamp, 12.5);
            }
            other => panic!("expected a frame, got {:?}", other),
        }
    }

    #[test]
    fn an_unconnected_shuffle_does_not_invalidate_the_graph() {
        let mut graph = Graph::new(4, 4);
        let shuffle_id = graph.add_node(Box::new(Shuffle::new()));

        graph.validate().expect("an unwired input is not a graph failure");

        RenderExecutor::new()
            .execute(&graph, shuffle_id, &context(4, 4))
            .expect("an unwired shuffle still renders");
    }

    #[test]
    fn a_source_wired_into_shuffle_is_read_through_the_graph() {
        let mut graph = Graph::new(1, 1);

        let mut source = ImageSource::new();
        source.set_image(image(vec![10, 20, 30, 40], 1, 1));

        let source_id = graph.add_node(Box::new(source));

        let mut shuffle = Shuffle::new();
        shuffle.set_parameter("RED_CHANNEL", Value::Text("BLUE".into())).unwrap();

        let shuffle_id = graph.add_node(Box::new(shuffle));

        graph
            .connect(shuffle_id, Input::Source, source_id)
            .expect("a source should connect to shuffle");

        graph.validate().expect("a wired graph is valid");

        let values = RenderExecutor::new()
            .execute(&graph, shuffle_id, &context(1, 1))
            .expect("the wired graph should render");

        match &values[0] {
            Value::Image(output) => assert_eq!(output.pixels, vec![30, 20, 30, 40]),
            other => panic!("expected an image, got {:?}", other),
        }
    }
}
