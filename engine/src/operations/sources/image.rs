// src/operations/sources/image.rs
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

pub struct ImageSource {
    pub image: Option<Arc<Image>>,
}

impl ImageSource {
    pub fn new() -> Self {
        Self {
            image: None,
        }
    }
    pub fn set_image(  &mut self, image: Arc<Image>, ) {
        self.image = Some(image);
    }
}

impl Operation for ImageSource {
    
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "image_source",
            menu: "INPUT",
            label: "Load Image",
            action: "image_source_open",
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
            display_name: "Image Source",
            category: OperationCategory::Source,
            input_count: 0,
            outputs: vec![OutputKind::Image],
        }
    }


    fn execute( &self, _ctx: &Context, _inputs: &[(Input, Value)], ) -> Result<Vec<Value>, OperationError> {
        let image = self
            .image
            .clone()
            .ok_or_else(|| OperationError::SourceNotFound("Image not loaded".to_string()))?;

        Ok(vec![
            Value::Image(image)
        ])
    }
}