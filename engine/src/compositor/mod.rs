pub mod context;
pub mod error;
pub mod graph;
pub mod input;
pub mod metadata;
pub mod node;
pub mod operations;
pub mod value;

pub use input::Input;
pub use node::Node;
pub use operation::Operation;

/*
Core, kind-agnostic contract every concrete operation implements.

Value is a closed enum, not a Box<dyn Value> trait object - every
operation matches on it directly instead of downcasting, and the
payload-carrying variants hold an Arc so passing a value to N
consumers (or storing it - Ghost's history, CapturedFrame's captured
background) is a refcount bump, never a deep pixel copy.
*/


