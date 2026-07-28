use std::any::Any;
use crate::compositor::{
    context::Context,
    error::OperationError,
    input::Input,
    metadata::{ OperationMetadata, ParameterDescriptor },
    value::Value
};
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
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError>;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    /*
    Unlike parameters() below, every concrete Operation implements this
    - there's no meaningful "operation with no name/category/output
    type" the way there's legitimately "operation with no editable
    settings", so this has no default. Purpose: future automatic node
    menus/editors/inspectors driven by this instead of a hardcoded
    per-kind list in JavaScript.
    */
    fn metadata(&self) -> OperationMetadata;

    /*
    UI-facing live parameter editing that doesn't require the caller
    to know (or downcast to) the concrete Operation type - a generic
    counterpart to as_any_mut, which stays for internal Rust use (see
    Graph::operation_mut). Defaults to "no editable parameters", which
    is correct for most operations (sources, composites, CapturedFrame
    have nothing a UI would show a control for); a concrete type only
    overrides these three when it actually has settings.
    */
    fn parameters(&self) -> Vec<ParameterDescriptor> {
        Vec::new()
    }

    fn get_parameter(&self, _name: &str) -> Option<Value> {
        None
    }

    fn set_parameter(&mut self, name: &str, _value: Value) -> Result<(), OperationError> {
        Err(OperationError::UnknownParameter(name.to_string()))
    }
}
