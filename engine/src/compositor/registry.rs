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

    /// Create a new registry pre-populated with all inventory-registered operations
    pub fn with_inventory() -> Self {
        let mut registry = Self::new();
        crate::operations::inventory::populate_registry(&mut registry);
        registry
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

    /// Register all operations from inventory
    pub fn register_from_inventory(&mut self) {
        crate::operations::inventory::populate_registry(self);
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
