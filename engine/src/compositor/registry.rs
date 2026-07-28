// src/compositor/registry.rs

use crate::compositor::{
    Operation,
    OperationDescriptor,
};

pub struct RegisteredOperation {
    pub descriptor: OperationDescriptor,
    pub constructor: fn() -> Box<dyn Operation>,
}

pub struct OperationRegistry {
    operations: Vec<RegisteredOperation>,
}

impl OperationRegistry {

    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        constructor: fn() -> Box<dyn Operation>,
    ) {
        let operation = constructor();

        self.operations.push(RegisteredOperation {
            descriptor: operation.descriptor(),
            constructor,
        });
    }

    pub fn descriptors(&self) -> Vec<OperationDescriptor> {
        self.operations
            .iter()
            .map(|op| op.descriptor.clone())
            .collect()
    }

    pub fn create(
        &self,
        id: &str,
    ) -> Option<Box<dyn Operation>> {
        self.operations
            .iter()
            .find(|op| op.descriptor.id == id)
            .map(|op| (op.constructor)())
    }
}