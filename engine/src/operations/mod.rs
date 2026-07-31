// src/operations/mod.rs
pub mod compose;
pub mod converters;
pub mod generators;
pub mod inventory;
pub mod key;
pub mod register;
pub mod sources;
pub mod transform;

pub use crate::graphics::frame::Frame;
pub use inventory::{initialize_inventory, get_all_descriptors, get_constructor, create_operation, populate_registry};
