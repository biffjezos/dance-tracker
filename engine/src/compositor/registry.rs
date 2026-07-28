// src/compositor/registry.rs

use crate::compositor::{
    Operation,
    OperationDescriptor,
};

pub struct OperationRegistry {
    operations: Vec<Box<dyn Operation>>,
}

impl OperationRegistry {

    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }


    pub fn register(
        &mut self,
        operation: Box<dyn Operation>,
    ) {
        self.operations.push(operation);
    }


    pub fn descriptors(&self) -> Vec<OperationDescriptor> {
        self.operations
            .iter()
            .map(|operation| operation.descriptor())
            .collect()
    }
}