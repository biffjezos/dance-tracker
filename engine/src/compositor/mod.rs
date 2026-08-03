// src/compositor/mod.rs
 
pub mod bbox;
pub mod context;
pub mod error;
pub mod executors;
pub mod graph;
pub mod input;
pub mod metadata;
pub mod operations;
pub mod operation_descriptor;
pub mod system_inventory;
pub mod registry;
pub mod system;
pub mod value;

pub use bbox::Rect;
pub use context::{ Context, ComputeMode, Meta };
pub use error::OperationError;
pub use input::Input;
pub use operations::Operation;

pub use operation_descriptor::OperationDescriptor;
pub use registry::OperationRegistry;
pub use value::{ Value, value_to_text };