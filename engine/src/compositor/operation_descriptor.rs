// src/compositor/operation_descriptor.rs
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct OperationDescriptor {
    pub id: &'static str,
    pub menu: &'static str,
    pub label: &'static str,
    pub action: Option<&'static str>,              // direct action
    pub ui_action: Option<&'static str>,          // UI-specific action (e.g., open file picker)
    pub buttons: &'static [OperationButton],       // submenu
}

#[derive(Clone, Debug, Serialize)]
pub struct OperationButton {
    pub label: &'static str,
    pub action: &'static str,
}
