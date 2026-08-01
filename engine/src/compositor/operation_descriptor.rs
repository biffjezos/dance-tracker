// src/compositor/operation_descriptor.rs
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct OperationDescriptor {
    pub id: &'static str,
    pub menu: &'static str,
    pub label: &'static str,
    pub action: Option<&'static str>,              // direct action
    pub ui_action: Option<&'static str>,          // UI-specific action (e.g., open file picker)
    pub create_node: Option<&'static str>,       // creates a graph operation node
    // Optional second-level grouping within `menu` - e.g. several
    // TRANSFORM operations sharing "SPECTRA" - purely presentational, no
    // engine meaning (unlike OperationCategory, which is about what kind
    // of thing an operation is). None means "no subgroup" - the menu
    // renders it as a direct button, same as before this field existed.
    pub submenu: Option<&'static str>,
}
