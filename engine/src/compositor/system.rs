use serde::Serialize;

use crate::compositor::{
    OperationDescriptor,
    metadata::ParameterDescriptor,
};

#[derive(Serialize, Clone)]
pub struct SystemMenuDescriptor {
    pub descriptor: OperationDescriptor,
    pub parameters: Vec<ParameterDescriptor>,
}

pub struct SystemMenu;

impl SystemMenu {
    pub fn descriptors() -> Vec<SystemMenuDescriptor> {
        crate::compositor::system_inventory::descriptors()
    }
}