// src/compositor/operations.rs

use std::any::Any;
use crate::compositor::{
    Context,
    OperationError,
    Input,
    OperationDescriptor,
    metadata::{ OperationMetadata, ParameterDescriptor },
    value::Value
};
use crate::compositor::;
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
}
