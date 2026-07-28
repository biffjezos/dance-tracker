use crate::compositor::value::Value;
/*
Named input slots, shared across every Operation instead of each one
inventing its own meaning for position 0 vs 1 (Compose's inputs[0]
being "foreground" was only ever a convention the caller and the
operation had to agree on separately). A Node's Vec<(Input, NodeId)>
labels each upstream wire with one of these, and the executors carry
the label through to the resolved Vec<(Input, Value)> an Operation
actually reads via find_input below.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Source,
    Reference,
    Content,
    Mask,
    Foreground,
    Background,
}

pub fn find_input(inputs: &[(Input, Value)], key: Input) -> Option<&Value> {
    inputs.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
}