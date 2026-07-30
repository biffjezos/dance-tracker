use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    input::find_input,
    Operation,
    OperationDescriptor,
    metadata::{
        OperationCategory,
        OperationMetadata,
        OutputKind,
        ParameterDescriptor,
        ParameterKind,
    },
    Value,
};

use crate::graphics::{Image, ImageFormat};

use crate::operations::transform::Multiply;

/**
move blendMode + impl Default for BlendMode + impl BlendMode
to src/graphics/..?
*/
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlendMode {
    Multiply,
}

pub const BLEND_MODES: &[&str] = &[
    "MULTIPLY",
];

impl Default for BlendMode {
    fn default() -> Self {
        BlendMode::Multiply
    }
}

impl BlendMode {

    pub fn to_str(&self) -> &'static str {
        match self {
            BlendMode::Multiply => "MULTIPLY",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.to_uppercase().as_str() {
            "MULTIPLY" => Some(BlendMode::Multiply),
            _ => None,
        }
    }
}

/// Merge operation - combines two inputs using a selected blend mode.
pub struct Merge {
    pub blend_mode: BlendMode,
}

impl Merge {

    pub fn new() -> Self {
        Self {
            blend_mode: BlendMode::Multiply,
        }
    }

    fn blend_pixels(
        mode: BlendMode,
        foreground: &[u8],
        background: &[u8],
    ) -> Vec<u8> {

        match mode {
            BlendMode::Multiply => {
                Multiply::multiply_pixels(
                    foreground,
                    background,
                )
            }
        }
    }

    fn image_from_value(
        value: &Value,
        ctx: &Context,
    ) -> Result<Arc<Image>, OperationError> {

        match value {

            Value::Image(image) => Ok(image.clone()),

            Value::Frame(frame) => Ok(Arc::new(Image {
                pixels: frame.pixels.clone(),
                width: frame.width,
                height: frame.height,
                format: ImageFormat::Rgba8,
            })),

            Value::Video(video) => {
                Ok(video.frame_at(ctx.meta.time)?)
            }

            other => Err(OperationError::InvalidInputType(
                format!("Merge cannot read {:?}", other)
            )),
        }
    }
}

impl Operation for Merge {

    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "merge",
            menu: "TRANSFORM",
            label: "MERGE",
            action: None,
            ui_action: None,
            create_node: Some("merge"),
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
            display_name: "Merge",
            category: OperationCategory::Color,
            inputs: vec![
                Input::Foreground,
                Input::Background,
            ],
            outputs: vec![
                OutputKind::Image,
            ],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "blend_mode",
                kind: ParameterKind::Enum(BLEND_MODES),
            }
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "blend_mode" => {
                Some(Value::Text(
                    self.blend_mode.to_str().to_string()
                ))
            }

            _ => None,
        }
    }

    fn set_parameter(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<(), OperationError> {

        match (name, value) {

            ("blend_mode", Value::Text(value)) => {

                self.blend_mode =
                    BlendMode::from_str(&value)
                    .ok_or_else(|| {
                        OperationError::InvalidParameterValue(
                            name.to_string(),
                            value,
                        )
                    })?;

                Ok(())
            }

            ("blend_mode", _) => {
                Err(OperationError::InvalidParameterType(
                    name.to_string()
                ))
            }

            _ => {
                Err(OperationError::UnknownParameter(
                    name.to_string()
                ))
            }
        }
    }

    fn execute(
        &self,
        ctx: &Context,
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {

        let Some(foreground) =
            find_input(inputs, Input::Foreground)
        else {
            return Err(OperationError::InvalidInputType(
                "Merge requires foreground input".into()
            ));
        };

        let Some(background) =
            find_input(inputs, Input::Background)
        else {
            return Err(OperationError::InvalidInputType(
                "Merge requires background input".into()
            ));
        };


        let foreground_image =
            Self::image_from_value(foreground, ctx)?;

        let background_image =
            Self::image_from_value(background, ctx)?;


        if foreground_image.width != background_image.width ||
           foreground_image.height != background_image.height {

            return Err(OperationError::InvalidInputType(
                "Merge inputs must have matching dimensions".into()
            ));
        }


        Ok(vec![
            Value::Image(Arc::new(Image {
                pixels: Self::blend_pixels(
                    self.blend_mode,
                    &foreground_image.pixels,
                    &background_image.pixels,
                ),
                width: foreground_image.width,
                height: foreground_image.height,
                format: foreground_image.format,
            }))
        ])
    }
}


// Inventory registration for Merge
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Merge::new())
    }
              }
