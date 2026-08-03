// src/compositor/system.rs
use serde::Serialize;

use crate::compositor::{
    OperationDescriptor,
    metadata::{ParameterDescriptor, ParameterKind},
};

pub struct SystemMenu;

impl SystemMenu {
    pub fn descriptors() -> Vec<SystemMenuDescriptor> {
        crate::compositor::system_inventory::descriptors()
    }
}

inventory::submit! {
    crate::compositor::system_inventory::SystemMenuInfo {
        constructor: || SystemMenuDescriptor {
            descriptor: OperationDescriptor {
                id: "compute_mode",
                menu: "PROJECT",
                label: "COMPUTE MODE",
                action: Some("compute_mode"),
                ui_action: None,
                create_node: None,
                submenu: Some("SETTINGS"),
            },
            parameters: vec![
                ParameterDescriptor {
                    name: "MODE",
                    kind: ParameterKind::Enum(&[
                        "CPU",
                        "GPU",
                        "AUTO",
                    ]),
                    group: None,
                }
            ],
        },
    }
}