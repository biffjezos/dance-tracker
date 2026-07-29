// src/operations/register.rs
use crate::compositor::OperationRegistry;

/// Register all operations from inventory into the registry
/// This is the new way to register operations - it uses the inventory crate
/// to automatically discover all operations that have been submitted via
/// inventory::submit! macros.
pub fn register_operations(registry: &mut OperationRegistry) {
    // Populate the registry from inventory
    registry.register_from_inventory();
}

/// Legacy function for backward compatibility
/// Creates a new registry and populates it from inventory
pub fn create_registry_with_operations() -> OperationRegistry {
    let mut registry = OperationRegistry::new();
    register_operations(&mut registry);
    registry
}
