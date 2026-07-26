/*
Core, kind-agnostic contract every concrete operation implements.

Value is a closed enum, not a Box<dyn Value> trait object - every
operation matches on it directly instead of downcasting, and the
payload-carrying variants hold an Arc so passing a value to N
consumers (or storing it - Ghost's history, CapturedFrame's captured
background) is a refcount bump, never a deep pixel copy.
*/

use std::any::Any;
use std::sync::Arc;

use crate::operations::{Frame, Image, Mask};

pub enum Value {
    Frame(Arc<Frame>),
    Mask(Arc<Mask>),
    Image(Arc<Image>),
    Number(f64),
    Boolean(bool),
    Text(String),
}

pub struct Context {
    pub data: Box<dyn Any + Send + Sync>,
}

#[derive(Debug)]
pub enum OperationError {
    MissingInput,
    WrongValueType,
    DimensionMismatch,
    SourceNotFound(String),
}

/*
as_any/as_any_mut aren't in the original stub - added so the
wasm-bindgen layer can downcast a Box<dyn Operation> back to its
concrete type (Text, Chroma, ...) to update live-editable parameters
(THRESHOLD +/-, text content, ...) on a node that already exists,
rather than needing every such edit to remove and recreate the node.
Rust can't provide a generic default for these on a trait object (the
classic object-safe "as_any" problem), so every impl repeats the same
three-line body. This is a different downcast from Value's - it's
about reaching a concrete Operation, not about reading a graph value,
so it stays even though Value itself no longer needs one.
*/
pub trait Operation: Any {
    fn execute(
        &self,
        ctx: &Context,
        inputs: &[Value],
    ) -> Result<Vec<Value>, OperationError>;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}
