use crate::compositor::{
    Context,
    OperationError,
    Value,
    graph::{Graph, NodeId},
};

mod preview;
mod render;

pub use preview::PreviewExecutor;
pub use render::RenderExecutor;

pub trait Execute {
    fn execute(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Vec<Value>, OperationError>;
}