pub mod context;
pub mod error;
pub mod graph;
pub mod input;
pub mod metadata;
pub mod operations;
pub mod value;

/*
Core, kind-agnostic contract every concrete operation implements.

Value is a closed enum, not a Box<dyn Value> trait object - every
operation matches on it directly instead of downcasting, and the
payload-carrying variants hold an Arc so passing a value to N
consumers (or storing it - Ghost's history, CapturedFrame's captured
background) is a refcount bump, never a deep pixel copy.
*/


