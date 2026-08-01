// src/operations/inventory.rs
// Inventory-based operation registration system

use std::sync::OnceLock;
use crate::compositor::{Operation, OperationDescriptor, OperationRegistry};

/// Type alias for operation constructors
pub type OperationConstructor = fn() -> Box<dyn Operation>;

/// Struct to hold registered operation information
#[derive(Debug)]
pub struct RegisteredOperationInfo {
    pub constructor: OperationConstructor,
    pub descriptor: OperationDescriptor,
}

/// Inventory substrate for operations
/// This uses the inventory crate to collect all operations at compile time
inventory::collect!(OperationInfo);

/// Static storage for all registered operations
static OPERATIONS: OnceLock<Vec<RegisteredOperationInfo>> = OnceLock::new();

/// Initialize the operation inventory
/// This should be called once at application startup
pub fn initialize_inventory() -> &'static Vec<RegisteredOperationInfo> {
    OPERATIONS.get_or_init(|| {
        // Collect all operations from inventory
        let mut operations = Vec::new();
        
        // Iterate through all collected OperationInfo entries
        for info in inventory::iter::<OperationInfo> {
            operations.push(RegisteredOperationInfo {
                constructor: info.constructor,
                descriptor: (info.constructor)().descriptor(),
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

/// Populate an OperationRegistry from inventory
pub fn populate_registry(registry: &mut OperationRegistry) {
    for info in initialize_inventory() {
        registry.register(info.constructor);
    }
}

/// What every operation file submits directly via `inventory::submit! { OperationInfo { ... } }`.
#[derive(Debug)]
pub struct OperationInfo {
    pub constructor: OperationConstructor,
}
