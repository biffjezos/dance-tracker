// src/operations/sources/video.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    Operation,
    OperationDescriptor,
    metadata::{ OperationCategory, OperationMetadata, OutputKind },
    Value
};

use crate::graphics::Image;

pub struct VideoSource {
    pub image: Option<Arc<Image>>,
}

impl VideoSource {
    pub fn new() -> Self {
        Self {
            image: None,
        }
    }
    
    pub fn set_image(&mut self, image: Arc<Image>) {
        self.image = Some(image);
    }
    
    pub fn get_image(&self) -> Option<Arc<Image>> {
        self.image.clone()
    }
}

impl Operation for VideoSource {
    
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "video_source",
            menu: "INPUT",
            label: "LOAD VIDEO",
            action: None,
            ui_action: Some("open_video_picker"),
            create_node: None,
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
            display_name: "Video Source",
            category: OperationCategory::Source,
            input_count: 0,
            outputs: vec![OutputKind::Image],
        }
    }


    fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let image = self
            .image
            .clone()
            .ok_or_else(|| OperationError::SourceNotFound("Video not loaded".to_string()))?;

        // Return Value::Image - conversion to Frame happens at the boundary (preview/render)
        Ok(vec![
            Value::Image(image)
        ])
    }
}

// Inventory registration for VideoSource
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(VideoSource::new())
    }
}
