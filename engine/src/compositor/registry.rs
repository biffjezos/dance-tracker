// src/compositor/registry.rs

use crate::compositor::{
    metadata::OperationCategory,
    Operation,
    OperationDescriptor,
};

pub struct RegisteredOperation {
    pub descriptor: OperationDescriptor,
    // Captured from the same throwaway instance descriptor() is read from,
    // at registration time - so callers that just want "what kind of thing
    // is this" (e.g. a future generic menu grouping) don't need to
    // construct an operation just to ask its metadata().
    pub category: OperationCategory,
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

    /// Register a constructor whose descriptor/category aren't known yet -
    /// constructs one throwaway instance to ask.
    pub fn register(
        &mut self,
        constructor: fn() -> Box<dyn Operation>,
    ) {
        let operation = constructor();
        self.register_with(constructor, operation.descriptor(), operation.metadata().category);
    }

    /// Register a constructor whose descriptor/category are already known
    /// (e.g. inventory already constructed one instance of its own to
    /// build its cached list) - skips constructing a second throwaway
    /// instance just to ask the same two questions again.
    pub fn register_with(
        &mut self,
        constructor: fn() -> Box<dyn Operation>,
        descriptor: OperationDescriptor,
        category: OperationCategory,
    ) {
        self.operations.push(RegisteredOperation {
            descriptor,
            category,
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

    /// Every registered operation's descriptor paired with its category -
    /// the JS-facing view a future generic menu grouping (Phase 4) would
    /// read instead of only the hand-maintained `menu` string.
    pub fn describe_all(&self) -> Vec<(OperationDescriptor, OperationCategory)> {
        self.operations
            .iter()
            .map(|op| (op.descriptor.clone(), op.category))
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
