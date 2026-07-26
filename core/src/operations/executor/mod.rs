use crate::compositor::{Context, OperationError, Value};
use crate::graph::{Graph, NodeId};

pub trait Execute {
    fn execute(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Vec<Value>, OperationError>;
}

pub mod simple;
pub mod preview;
pub mod render;

pub use simple::SimpleExecutor;
pub use preview::PreviewExecutor;
pub use render::RenderExecutor;
