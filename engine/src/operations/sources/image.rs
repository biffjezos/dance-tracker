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
    pub image: Arc<Image>,
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


    fn execute(
        &self,
        _ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {

        Ok(vec![
            Value::Image(self.image.clone())
        ])
    }
}