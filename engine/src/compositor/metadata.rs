
use crate::compositor::input::Input;

/*
Only the scalar Value variants make sense as something a UI would show
a control for - Frame/Mask/Image are graph-wired inputs, never a
setting on the node that produces them.
*/
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParameterKind {
    /// A stepper's increment and, optionally, its bounds - the operation
    /// owns them, so a UI stepper moves by exactly this amount and cannot
    /// be pushed past a bound the operation itself would reject.
    Number { step: f64, min: Option<f64>, max: Option<f64> },
    Boolean,
    Text,
    Color,
    /// A closed set of values. The operation owns the list, so a UI selector
    /// offers exactly these and nothing else.
    Enum(&'static [&'static str])
}

impl ParameterKind {
    /// The values a selector may offer for this parameter - empty for
    /// parameters that are not a closed set.
    pub fn options(&self) -> &'static [&'static str] {
        match self {
            ParameterKind::Enum(options) => options,
            _ => &[],
        }
    }

    /// The stepper increment for a Number parameter - None for every other kind.
    pub fn step(&self) -> Option<f64> {
        match self {
            ParameterKind::Number { step, .. } => Some(*step),
            _ => None,
        }
    }

    /// The lower bound a Number parameter accepts, if any.
    pub fn min(&self) -> Option<f64> {
        match self {
            ParameterKind::Number { min, .. } => *min,
            _ => None,
        }
    }

    /// The upper bound a Number parameter accepts, if any.
    pub fn max(&self) -> Option<f64> {
        match self {
            ParameterKind::Number { max, .. } => *max,
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ParameterKind::Number { .. } => "NUMBER",
            ParameterKind::Boolean => "BOOLEAN",
            ParameterKind::Text => "TEXT",
            ParameterKind::Color => "COLOR",
            ParameterKind::Enum(_) => "ENUM",
        }
    }
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
    Video,
    Number,
    Boolean,
    Text,
    Color
}

#[derive(Clone, Debug)]
pub struct OperationMetadata {
    pub display_name: &'static str,
    pub category: OperationCategory,
    /// Which inputs this operation can be wired to, in menu order. A source
    /// operation declares none; anything the UI offers to wire comes from here,
    /// never from a hardcoded list on the UI side.
    pub inputs: Vec<Input>,
    pub outputs: Vec<OutputKind>,
}

impl OperationMetadata {
    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }
}
