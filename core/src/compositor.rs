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

pub trait Operation {
    fn execute(
        &self,
        ctx: &Context,
        inputs: &[Box<dyn Value>],
    ) -> Result<Vec<Box<dyn Value>>, OperationError>;
}
