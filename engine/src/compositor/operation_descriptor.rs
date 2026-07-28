// src/compositor/operation_descriptor.rs
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct OperationDescriptor {
    pub id: &'static str,
    pub menu: &'static str,
    pub label: &'static str,
    pub buttons: &'static [OperationButton],
}

#[derive(Clone, Debug, Serialize)]
pub struct OperationButton {
    pub label: &'static str,
    pub action: &'static str,
}