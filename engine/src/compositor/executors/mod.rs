mod preview;
mod render;

pub use preview::PreviewExecutor;
pub use render::RenderExecutor;

use crate::compositor::{
    Context,
    OperationError,
    Value,
    graph::{Graph, NodeId},
};




pub trait Execute {
    fn execute(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Vec<Value>, OperationError>;
}