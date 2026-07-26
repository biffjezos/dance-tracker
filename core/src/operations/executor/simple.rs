use crate::compositor::{Context, OperationError, Value};
use crate::graph::{Graph, NodeId};
use crate::operations::executor::Execute;

/*
Unchanged from the stub. Recomputes every input on every call, with no
memoization - correct for a one-off (a control action, a single
preview pull) but wasteful if the same node feeds two consumers (e.g.
one VideoSource feeding both a Compose and a mask), since it would run
twice. RenderExecutor is where that gets addressed once the full
per-frame graph walk exists.
*/
pub struct SimpleExecutor;

impl Execute for SimpleExecutor {
    fn execute(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Vec<Value>, OperationError> {
        let node = &graph.nodes[node];

        let mut input_values = Vec::new();

        for input_node_id in &node.inputs {
            let values = self.execute(graph, *input_node_id, ctx)?;
            input_values.push(values.into_iter().next().unwrap());
        }

        node.operation.execute(ctx, &input_values)
    }
}
