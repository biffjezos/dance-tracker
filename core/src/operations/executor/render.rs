use std::collections::HashMap;

use crate::compositor::{Context, Input, OperationError, Value};
use crate::graph::{Graph, NodeId};
use crate::operations::executor::Execute;

/*
Drives the main render/output canvas - confirmed as one of the two
real-time executors (the other being PreviewExecutor), each tick
evaluating the graph's final output node and writing the result to the
output canvas (see dom::write_frame_to_canvas). SimpleExecutor is for
one-off, non-per-frame evaluations instead (controls, ad hoc calls).

Per-tick memoization (memo is fresh every execute() call, never
persisted across ticks - values legitimately change every frame, e.g.
a live video's current pixels). Confirmed real, not hypothetical: the
full render graph combines every independently-visible thing, and
app.js's own wiring lets a video be both independently visible *and*
the Input::Source feeding a mask's Chroma/Difference - the exact
"video that's both shown raw and keyed" case in the app - so a shared
source node genuinely gets reached twice within one traversal. Left as
plain recursion in PreviewExecutor and SimpleExecutor, where fan-out
either can't happen (PreviewExecutor evaluates a single selected
node's own dependency chain, typically linear) or doesn't matter (a
one-off pull).
*/
pub struct RenderExecutor;

impl Execute for RenderExecutor {
    fn execute(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Vec<Value>, OperationError> {
        let mut memo = HashMap::new();
        let value = Self::evaluate(graph, node, ctx, &mut memo)?;
        Ok(vec![value])
    }
}

impl RenderExecutor {
    fn evaluate(
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
        memo: &mut HashMap<NodeId, Value>,
    ) -> Result<Value, OperationError> {
        if let Some(cached) = memo.get(&node) {
            return Ok(cached.clone());
        }

        let node_data = graph.resolve(node).ok_or(OperationError::UnknownNode)?;

        let mut input_values: Vec<(Input, Value)> = Vec::new();

        for &(key, input_node_id) in &node_data.inputs {
            let value = Self::evaluate(graph, input_node_id, ctx, memo)?;
            input_values.push((key, value));
        }

        let outputs = node_data.operation.execute(ctx, &input_values)?;
        let value = outputs.into_iter().next().unwrap();

        memo.insert(node, value.clone());

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;
    use std::cell::Cell;

    use crate::compositor::{OperationCategory, OperationMetadata};
    use crate::graph::Node;
    use crate::operations::executor::Execute;

    struct CountingSource {
        calls: Cell<u32>,
    }

    impl crate::compositor::Operation for CountingSource {
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "CountingSource",
                category: OperationCategory::Source,
                input_count: 0,
                outputs: vec![],
            }
        }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            self.calls.set(self.calls.get() + 1);
            Ok(vec![Value::Number(1.0)])
        }
    }

    struct Combine;

    impl crate::compositor::Operation for Combine {
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "Combine",
                category: OperationCategory::Composite,
                input_count: 2,
                outputs: vec![],
            }
        }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![Value::Number(2.0)])
        }
    }

    #[test]
    fn a_node_feeding_two_consumers_is_evaluated_once_per_tick() {
        let mut graph = Graph::new(1, 1);

        let source_id = graph.add_node(Node {
            operation: Box::new(CountingSource { calls: Cell::new(0) }),
            inputs: vec![],
        });

        let combine_id = graph.add_node(Node {
            operation: Box::new(Combine),
            inputs: vec![(Input::Foreground, source_id), (Input::Background, source_id)],
        });

        let ctx = Context::default();
        RenderExecutor.execute(&graph, combine_id, &ctx).expect("should succeed");

        let calls = graph
            .resolve(source_id)
            .unwrap()
            .operation
            .as_any()
            .downcast_ref::<CountingSource>()
            .unwrap()
            .calls
            .get();

        assert_eq!(calls, 1, "shared source should be evaluated once, not once per consumer");
    }

    #[test]
    fn a_node_feeding_two_consumers_is_evaluated_fresh_on_the_next_tick() {
        let mut graph = Graph::new(1, 1);

        let source_id = graph.add_node(Node {
            operation: Box::new(CountingSource { calls: Cell::new(0) }),
            inputs: vec![],
        });

        let combine_id = graph.add_node(Node {
            operation: Box::new(Combine),
            inputs: vec![(Input::Foreground, source_id), (Input::Background, source_id)],
        });

        let ctx = Context::default();
        RenderExecutor.execute(&graph, combine_id, &ctx).expect("should succeed");
        RenderExecutor.execute(&graph, combine_id, &ctx).expect("should succeed");

        let calls = graph
            .resolve(source_id)
            .unwrap()
            .operation
            .as_any()
            .downcast_ref::<CountingSource>()
            .unwrap()
            .calls
            .get();

        assert_eq!(calls, 2, "memoization must not persist across separate execute() calls");
    }
}
