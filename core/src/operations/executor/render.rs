use crate::compositor::{Context, Input, OperationError, Value};
use crate::graph::{Graph, NodeId};
use crate::operations::executor::Execute;

/*
Drives the main render/output canvas - confirmed as one of the two
real-time executors (the other being PreviewExecutor), each tick
evaluating the graph's final output node and writing the result to the
output canvas (see dom::write_frame_to_canvas). SimpleExecutor is for
one-off, non-per-frame evaluations instead (controls, ad hoc calls).

Functionally identical to SimpleExecutor for now. Likely next need:
per-tick memoization - the full render graph is far more likely than a
preview to have one node (a source, a generator) feeding more than one
consumer, and this naive recursion would recompute it once per
consumer instead of once per frame. Left as plain recursion until the
real graph shapes (and how often that fan-out actually happens) are
known, rather than guessing a caching strategy now.
*/
pub struct RenderExecutor;

impl Execute for RenderExecutor {
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
