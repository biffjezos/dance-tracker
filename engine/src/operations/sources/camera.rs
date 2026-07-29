// src/operations/sources/camera.rs
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

/// Camera source operation (stub implementation)
pub struct CameraSource {
    // Placeholder for camera state
}

impl CameraSource {
    pub fn new() -> Self {
        Self {
            // Initialize camera state
        }
    }
}

impl Operation for CameraSource {
    
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "camera_source",
            menu: "INPUT",
            label: "CAMERA",
            action: None,
            ui_action: Some("open_camera_picker"),
            create_node: Some("camera_source"),
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
            display_name: "Camera Source",
            category: OperationCategory::Source,
            input_count: 0,
            outputs: vec![OutputKind::Image],
        }
    }

    fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        // Stub implementation - returns an error for now
        Err(OperationError::NotImplemented("Camera source not yet implemented".to_string()))
    }
}

// Inventory registration for CameraSource
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(CameraSource::new())
    }
}
