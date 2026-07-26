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

pub enum Value {
    Frame(Arc<Frame>),
    Mask(Arc<Mask>),
    Image(Arc<Image>),
    Number(f64),
    Boolean(bool),
    Text(String),
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
render pass, how much fidelity to spend. Distinct from ResourceManager
below: Meta is cheap and rebuilt fresh every tick, ResourceManager is
the persistent, shared part of Context.
*/
#[derive(Clone, Copy, Debug, Default)]
pub struct Meta {
    pub frame: u64,
    pub fps: f32,
    pub time: f64,
    pub preview: bool,
    pub render_quality: RenderQuality,
}

#[derive(Clone, Default)]
pub struct Context {
    pub meta: Meta,
    pub resources: ResourceManager,
}

#[derive(Debug)]
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
