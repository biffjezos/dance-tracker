// src/operations/inventory.rs
// Inventory-based operation registration system

use std::sync::OnceLock;
use crate::compositor::{Operation, OperationDescriptor, OperationRegistry};
use crate::compositor::metadata::OperationCategory;

/// Type alias for operation constructors
pub type OperationConstructor = fn() -> Box<dyn Operation>;

/// Struct to hold registered operation information
#[derive(Debug)]
pub struct RegisteredOperationInfo {
    pub constructor: OperationConstructor,
    pub descriptor: OperationDescriptor,
    pub category: OperationCategory,
}

/// Inventory substrate for operations
/// This uses the inventory crate to collect all operations at compile time
inventory::collect!(OperationInfo);

/// Static storage for all registered operations
static OPERATIONS: OnceLock<Vec<RegisteredOperationInfo>> = OnceLock::new();

/// Initialize the operation inventory (once, cached for the process's
/// lifetime - constructing one throwaway instance per operation type here
/// is what lets every other function in this file, and OperationRegistry::
/// register_with, read descriptor/category back without constructing
/// their own instance just to ask the same two questions again).
/// This should be called once at application startup.
pub fn initialize_inventory() -> &'static Vec<RegisteredOperationInfo> {
    OPERATIONS.get_or_init(|| {
        let mut operations = Vec::new();

        for info in inventory::iter::<OperationInfo> {
            let operation = (info.constructor)();
            operations.push(RegisteredOperationInfo {
                constructor: info.constructor,
                descriptor: operation.descriptor(),
                category: operation.metadata().category,
            });
        }

        operations
    })
}

/// Get all registered operation descriptors
pub fn get_all_descriptors() -> Vec<OperationDescriptor> {
    initialize_inventory()
        .iter()
        .map(|op| op.descriptor.clone())
        .collect()
}

/// Get a specific operation constructor by ID
pub fn get_constructor(id: &str) -> Option<OperationConstructor> {
    initialize_inventory()
        .iter()
        .find(|op| op.descriptor.id == id)
        .map(|op| op.constructor)
}

/// Create an operation by ID
pub fn create_operation(id: &str) -> Option<Box<dyn Operation>> {
    get_constructor(id).map(|constructor| constructor())
}

/// Populate an OperationRegistry from inventory. Uses each entry's already-
/// known descriptor/category (computed once in initialize_inventory above)
/// instead of constructing a second throwaway instance per operation.
pub fn populate_registry(registry: &mut OperationRegistry) {
    for info in initialize_inventory() {
        registry.register_with(info.constructor, info.descriptor.clone(), info.category);
    }
}

/// What every operation file submits directly via `inventory::submit! { OperationInfo { ... } }`.
#[derive(Debug)]
pub struct OperationInfo {
    pub constructor: OperationConstructor,
}
