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
use crate::resource_manager::ResourceManager;

#[derive(Debug)]
pub enum Value {
    Frame(Arc<Frame>),
    Mask(Arc<Mask>),
    Image(Arc<Image>),
    Number(f64),
    Boolean(bool),
    Text(String),
}

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

/*
Draft trades fidelity for speed (e.g. an operation could skip
antialiasing or subsample); Full is always correct. Nothing branches
on this yet - it exists so Meta's shape is already right for the first
operation that needs the distinction, same as Value::Mask/Image exist
ahead of anything producing them.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderQuality {
    Draft,
    Full,
}

impl Default for RenderQuality {
    fn default() -> Self {
        RenderQuality::Full
    }
}

/*
Per-tick, read-only facts about the evaluation that's currently
running - which frame/time this is, whether it's the preview or main
render pass, how much fidelity to spend, and the graph's current
render resolution (width/height mirror Graph::resolution() - see
App::context - so a source/generator operation reads its output size
from here every execute() call instead of baking one in at
construction time and needing the whole graph rebuilt to change it).
Distinct from ResourceManager below: Meta is cheap and rebuilt fresh
every tick, ResourceManager is the persistent, shared part of Context.
*/
#[derive(Clone, Copy, Debug, Default)]
pub struct Meta {
    pub frame: u64,
    pub fps: f32,
    pub time: f64,
    pub preview: bool,
    pub render_quality: RenderQuality,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Default)]
pub struct Context {
    pub meta: Meta,
    pub resources: ResourceManager,
}

#[derive(Debug, Clone)]
pub enum OperationError {
    MissingInput,
    WrongValueType,
    DimensionMismatch,
    SourceNotFound(String),
    /*
    The offending cycle, node ids in traversal order (last id repeats
    the first, closing the loop) - not crate::graph::NodeId, to avoid
    graph.rs and compositor.rs importing each other; the two are the
    same underlying usize.
    */
    Cycle(Vec<usize>),
    // set_parameter with a name the operation doesn't have - distinct
    // from WrongValueType, which is a real parameter given a value of
    // the wrong kind.
    UnknownParameter(String),
    // A NodeId that doesn't resolve - out of range, or a stale
    // generation (its node has since been removed). Distinct from
    // MissingInput, which is a wire that was never connected at all.
    UnknownNode,
}

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
}

#[derive(Clone, Debug)]
pub struct OperationMetadata {
    pub display_name: &'static str,
    pub category: OperationCategory,
    pub input_count: usize,
    pub outputs: Vec<OutputKind>,
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
