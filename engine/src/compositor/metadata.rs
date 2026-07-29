
/*
Only the scalar Value variants make sense as something a UI would show
a control for - Frame/Mask/Image are graph-wired inputs, never a
setting on the node that produces them.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterKind {
    Number,
    Boolean,
    Text,
    Color
}

#[derive(Clone, Debug)]
pub struct ParameterDescriptor {
    pub name: &'static str,
    pub kind: ParameterKind,
}

/*
What kind of thing an operation is, for grouping in a future automatic
node menu/editor - Reference covers CapturedFrame, a settable handle
rather than something that decodes, generates, keys, or composites.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationCategory {
    Source,
    Generator,
    Mask,
    Composite,
    Reference,
    Color,
}

/*
Which Value variant(s) an operation's execute() can return - every
operation here only ever produces exactly one output today, but this
is a Vec (not a single OutputKind) so a future multi-output operation
doesn't need the shape to change.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputKind {
    Frame,
    Mask,
    Image,
    Number,
    Boolean,
    Text,
    Color
}

#[derive(Clone, Debug)]
pub struct OperationMetadata {
    pub display_name: &'static str,
    pub category: OperationCategory,
    pub input_count: usize,
    pub outputs: Vec<OutputKind>,
}
