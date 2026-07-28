// src/operations/register.rs
use crate::compositor::OperationRegistry;
use crate::operations::sources::ImageSource;

pub fn register_operations(registry: &mut OperationRegistry,) {
    registry.register(|| Box::new(ImageSource::new()));
}