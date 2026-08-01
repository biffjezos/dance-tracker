// src/operations/sources/camera.rs
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

use crate::operations::sources::PixelSource;

/// Live camera source.
///
/// The host owns the capture device and hands the running stream over as a
/// PixelSource; this operation only pulls the current frame out of it.
pub struct CameraSource {
    pub pixel_source: Option<Arc<dyn PixelSource>>,
}

impl CameraSource {
    pub fn new() -> Self {
        Self {
            pixel_source: None,
        }
    }

    pub fn set_source(&mut self, source: Arc<dyn PixelSource>) {
        self.pixel_source = Some(source);
    }

    pub fn get_source(&self) -> Option<Arc<dyn PixelSource>> {
        self.pixel_source.clone()
    }
}

impl Operation for CameraSource {

    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "camera_source",
            menu: "INPUT",
            label: "CAMERA",
            action: None,
            ui_action: Some("open_camera_stream"),
            create_node: None,
            submenu: None,
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
            inputs: Vec::new(),
            outputs: vec![OutputKind::Frame],
        }
    }

    fn set_pixel_source(
        &mut self,
        source: Arc<dyn PixelSource>,
    ) -> Result<(), OperationError> {
        self.set_source(source);
        Ok(())
    }

    // A running camera stream produces a new frame every tick with neither
    // its (nonexistent) parameters nor its (nonexistent) inputs ever
    // changing - never safe to serve from RenderExecutor's cross-tick cache.
    fn is_live(&self) -> bool {
        true
    }

    fn execute(
        &self,
        ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        let source = self
            .pixel_source
            .as_ref()
            .ok_or_else(|| {
                OperationError::SourceNotFound(
                    "Camera stream not attached".to_string()
                )
            })?;

        let frame = source.read(
            ctx.meta.width,
            ctx.meta.height,
        )?;

        Ok(vec![
            Value::Frame(Arc::new(frame))
        ])
    }
}

// Inventory registration for CameraSource
inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(CameraSource::new())
    }
}
