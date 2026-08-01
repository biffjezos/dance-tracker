// src/operations/register.rs
use crate::compositor::OperationRegistry;

/// Register all operations from inventory into the registry. Uses the
/// inventory crate to automatically discover all operations that have
/// been submitted via inventory::submit! macros.
pub fn register_operations(registry: &mut OperationRegistry) {
    registry.register_from_inventory();
}
