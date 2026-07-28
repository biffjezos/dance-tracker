use crate::compositor::{Context, Input, OperationError, Value};
use crate::graph::{Graph, NodeId};
use crate::operations::executor::Execute;

/*
Drives the left preview/input canvas - confirmed as one of the two
real-time executors (the other being RenderExecutor), each tick pulling
whichever single node is currently selected and writing its result to
that canvas (see dom::write_frame_to_canvas). SimpleExecutor is for
one-off, non-per-frame evaluations instead (controls, ad hoc calls).

Functionally identical to SimpleExecutor for now (same plain recursive
walk, no caching) - still open: whether this ever needs caching at all,
given it's usually evaluating a much smaller subgraph than the full
render.
*/
pub struct PreviewExecutor;

impl Execute for PreviewExecutor {
    fn execute(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Vec<Value>, OperationError> {
        let node_data = graph.resolve(node).ok_or(OperationError::UnknownNode)?;

        let mut input_values: Vec<(Input, Value)> = Vec::new();

        for &(key, input_node_id) in &node_data.inputs {
            let values = self.execute(graph, input_node_id, ctx)?;
            input_values.push((key, values.into_iter().next().unwrap()));
        }

        node_data.operation.execute(ctx, &input_values)
    }
}