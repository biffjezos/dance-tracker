use std::sync::OnceLock;

use crate::compositor::system::SystemMenuDescriptor;

inventory::collect!(SystemMenuInfo);

static MENUS: OnceLock<Vec<SystemMenuDescriptor>> = OnceLock::new();

#[derive(Debug)]
pub struct SystemMenuInfo {
    pub constructor: fn() -> SystemMenuDescriptor,
}

pub fn initialize_inventory() -> &'static Vec<SystemMenuDescriptor> {
    MENUS.get_or_init(|| {
        inventory::iter::<SystemMenuInfo>
            .into_iter()
            .map(|m| (m.constructor)())
            .collect()
    })
}

pub fn descriptors() -> Vec<SystemMenuDescriptor> {
    initialize_inventory().clone()
}