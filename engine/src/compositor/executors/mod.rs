use crate::compositor::{
    Context,
    OperationError,
    Value,
    graph::{Graph, NodeId},
};

mod simple;
mod preview;
mod render;

pub use simple::SimpleExecutor;
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