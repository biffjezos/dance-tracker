// src/operations/register.rs
use crate::compositor::OperationRegistry;
use crate::operations::sources::ImageSource;
use crate::operations::transform::Shuffle;

pub fn register_operations(registry: &mut OperationRegistry,) {
    registry.register(|| Box::new(ImageSource::new()));
    registry.register(|| Box::new(Shuffle::new()));
}
