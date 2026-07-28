// src/compositor/operation_descriptor.rs

#[derive(Clone, Debug)]
pub struct OperationDescriptor {
    pub id: &'static str,
    pub menu: &'static str,
    pub label: &'static str,
    pub buttons: &'static [OperationButton],
}

#[derive(Clone, Debug)]
pub struct OperationButton {
    pub label: &'static str,
    pub action: &'static str,
}