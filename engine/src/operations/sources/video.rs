// src/operations/sources/video.rs
use std::any::Any;

use crate::compositor::{
    Context,
    OperationError,
    Input,
    Operation,
    OperationDescriptor,
    metadata::{ OperationCategory, OperationMetadata, OutputKind },
    Value
};

/// Video source operation (stub implementation)
pub struct VideoSource {
    // Placeholder for video state
}

impl VideoSource {
    pub fn new() -> Self {
        Self {
            // Initialize video state
        }
    }
}

impl Operation for VideoSource {
    
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "video_source",
            menu: "INPUT",
            label: "VIDEO",
            action: None,
            ui_action: Some("open_video_picker"),
            create_node: Some("video_source"),
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
        // Stub implementation - returns an error for now
        Err(OperationError::NotImplemented("Video source not yet implemented".to_string()))
    }
}

// Inventory registration for VideoSource
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(VideoSource::new())
    }
}
