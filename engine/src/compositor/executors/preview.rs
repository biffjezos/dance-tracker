// src/compositor/executors/preview.rs

use crate::compositor::{Context, Input, OperationError, Value};
use crate::compositor::graph::{Graph, NodeId};
use super::Execute;

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