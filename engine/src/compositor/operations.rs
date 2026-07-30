// src/compositor/operations.rs

use std::any::Any;
use std::sync::Arc;

use crate::operations::sources::PixelSource;
use crate::compositor::{
    Context,
    OperationDescriptor,
    OperationError,
    input::Input,
    metadata::{ OperationMetadata, ParameterDescriptor },
    Value
};
pub trait Operation: Any {
    fn descriptor(&self) -> OperationDescriptor;
    
    fn execute(
        &self,
        ctx: &Context,
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError>;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn metadata(&self) -> OperationMetadata;

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        Vec::new()
    }

    fn get_parameter(&self, _name: &str) -> Option<Value> {
        None
    }

    fn set_parameter(&mut self, name: &str, _value: Value) -> Result<(), OperationError> {
        Err(OperationError::UnknownParameter(name.to_string()))
    }

    /// Returns true if this operation supports editing (has parameters that can be modified)
    fn supports_edit(&self) -> bool {
        !self.parameters().is_empty()
    }

    /// Attach a live pixel source to this operation.
    ///
    /// Source operations that pull their pixels from something the host owns
    /// (a camera stream, a decoded video element) accept one here, so the
    /// host side never needs to know which concrete operation it is talking to.
    fn set_pixel_source(
        &mut self,
        _source: Arc<dyn PixelSource>,
    ) -> Result<(), OperationError> {
        Err(OperationError::NotImplemented(
            format!(
                "{} does not read from a pixel source",
                self.metadata().display_name
            )
        ))
    }
}
