/*
Core, kind-agnostic contract every concrete operation implements -
unchanged from the stub. Value is a blanket-impl marker over any
concrete type (Frame, a number, a control signal, ...) so the graph
can carry more than pixels without every Operation agreeing on one
payload type up front; a caller downcasts (std::any::Any, via trait
upcasting: dyn Value -> dyn Any, since Value: Any) to whatever
concrete type it actually expects from a given input.
*/

use std::any::Any;

pub trait Value: Any + Send + Sync {}

impl<T: Any + Send + Sync> Value for T {}

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
three-line body.
*/
pub trait Operation: std::any::Any {
    fn execute(
        &self,
        ctx: &Context,
        inputs: &[Box<dyn Value>],
    ) -> Result<Vec<Box<dyn Value>>, OperationError>;

    fn as_any(&self) -> &dyn std::any::Any;

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
