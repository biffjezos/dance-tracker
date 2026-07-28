// src/compositor/executors/simple.rs

use crate::compositor::{Context, Input, OperationError, Value};
use crate::compositor::graph::{Graph, NodeId};
use super::Execute;

pub struct SimpleExecutor;

impl Execute for SimpleExecutor {
    fn execute(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Vec<Value>, OperationError> {
        let node = graph.resolve(node).ok_or(OperationError::UnknownNode)?;

        let mut input_values: Vec<(Input, Value)> = Vec::new();

        for &(key, input_node_id) in &node.inputs {
            let values = self.execute(graph, input_node_id, ctx)?;
            input_values.push((key, values.into_iter().next().unwrap()));
        }

        node.operation.execute(ctx, &input_values)
    }
}