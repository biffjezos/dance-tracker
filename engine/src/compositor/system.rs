use serde::Serialize;

use crate::compositor::{
    OperationDescriptor,
    metadata::{ParameterDescriptor, ParameterKind},
};

pub struct SystemMenu;

#[derive(Serialize)]
pub struct SystemMenuDescriptor {
    pub descriptor: OperationDescriptor,
    pub parameters: Vec<ParameterDescriptor>,
}

impl SystemMenu {
    pub fn descriptors() -> Vec<SystemMenuDescriptor> {
        vec![
            SystemMenuDescriptor {
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
            }
        ]
    }
}