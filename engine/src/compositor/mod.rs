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
pub mod registry;
pub mod value;

pub mod system_menu;

pub use bbox::Rect;
pub use context::{ Context, Meta };
pub use error::OperationError;
pub use input::Input;
pub use operations::Operation;

pub use operation_descriptor::OperationDescriptor;
pub use registry::OperationRegistry;
pub use value::{ Value, value_to_text };