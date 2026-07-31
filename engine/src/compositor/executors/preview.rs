// src/compositor/executors/preview.rs

use std::collections::HashMap;

use crate::compositor::{Context, Input, OperationError, Value};
use crate::compositor::graph::{Graph, NodeId};
use super::Execute;

/// `memoize: true` dedupes a node shared by more than one downstream
/// consumer (e.g. a mask wired into two composite branches) so it's only
/// evaluated once per call, instead of once per consumer - without this, a
/// live source read on each of those redundant paths can each land on a
/// very slightly different instant, producing a visibly inconsistent
/// composite. `false` restores the original behaviour (every path always
/// recomputed independently) for comparison/debugging.
///
/// Neither setting persists a cache *across* separate execute() calls -
/// that's RenderExecutor's job. A preview is always fresh tick to tick;
/// `memoize` only controls deduping *within* one tick's own evaluation.
pub struct PreviewExecutor {
    pub memoize: bool,
}

impl PreviewExecutor {
    pub fn new(memoize: bool) -> Self {
        Self { memoize }
    }
}

impl Default for PreviewExecutor {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Execute for PreviewExecutor {
    fn execute(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Vec<Value>, OperationError> {
        if !self.memoize {
            return Self::evaluate_unmemoized(graph, node, ctx).map(|value| vec![value]);
        }

        let mut memo = HashMap::new();
        Self::evaluate_memoized(graph, node, ctx, &mut memo).map(|value| vec![value])
    }
}

impl PreviewExecutor {
    fn evaluate_unmemoized(
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Value, OperationError> {
        let node_data = graph.resolve(node).ok_or(OperationError::UnknownNode)?;

        let mut input_values: Vec<(Input, Value)> = Vec::new();

        for &(key, input_node_id) in &node_data.inputs {
            let value = Self::evaluate_unmemoized(graph, input_node_id, ctx)?;
            input_values.push((key, value));
        }

        let outputs = node_data.operation.execute(ctx, &input_values)?;
        // ok_or (not unwrap) so an operation that violates the "always
        // returns exactly one output" convention errors out this one
        // preview instead of panicking the whole WASM instance.
        outputs.into_iter().next().ok_or(OperationError::NoOutput)
    }

    fn evaluate_memoized(
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
            let value = Self::evaluate_memoized(graph, input_node_id, ctx, memo)?;
            input_values.push((key, value));
        }

        let outputs = node_data.operation.execute(ctx, &input_values)?;
        // See evaluate_unmemoized: ok_or, not unwrap, so a no-output
        // operation errors out this preview instead of panicking.
        let value = outputs.into_iter().next().ok_or(OperationError::NoOutput)?;

        memo.insert(node, value.clone());

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;
    use std::cell::Cell;

    use crate::compositor::graph::Graph;
    use crate::compositor::metadata::{OperationCategory, OperationMetadata};
    use crate::compositor::{Operation, OperationDescriptor};

    struct CountingSource {
        calls: Cell<u32>,
    }

    impl Operation for CountingSource {
        fn descriptor(&self) -> OperationDescriptor {
            OperationDescriptor {
                id: "counting_source",
                menu: "TEST",
                label: "COUNTING SOURCE",
                action: None,
                ui_action: None,
                create_node: None,
            }
        }

        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "CountingSource",
                category: OperationCategory::Source,
                inputs: vec![],
                outputs: vec![],
            }
        }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            self.calls.set(self.calls.get() + 1);
            Ok(vec![Value::Number(self.calls.get() as f64)])
        }
    }

    struct Combine;

    impl Operation for Combine {
        fn descriptor(&self) -> OperationDescriptor {
            OperationDescriptor {
                id: "combine",
                menu: "TEST",
                label: "COMBINE",
                action: None,
                ui_action: None,
                create_node: None,
            }
        }

        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "Combine",
                category: OperationCategory::Composite,
                inputs: vec![Input::Foreground, Input::Background],
                outputs: vec![],
            }
        }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![Value::Number(0.0)])
        }
    }

    fn context() -> Context {
        Context {
            meta: crate::compositor::Meta { width: 1, height: 1, ..Default::default() },
            ..Default::default()
        }
    }

    fn shared_source_graph() -> (Graph, NodeId, NodeId) {
        let mut graph = Graph::new(1, 1);
        let source_id = graph.add_node(Box::new(CountingSource { calls: Cell::new(0) }));
        let combine_id = graph.add_node(Box::new(Combine));
        graph.connect(combine_id, Input::Foreground, source_id).unwrap();
        graph.connect(combine_id, Input::Background, source_id).unwrap();
        (graph, source_id, combine_id)
    }

    fn calls_of(graph: &Graph, node: NodeId) -> u32 {
        graph
            .resolve(node)
            .unwrap()
            .operation
            .as_any()
            .downcast_ref::<CountingSource>()
            .unwrap()
            .calls
            .get()
    }

    #[test]
    fn memoize_true_evaluates_a_shared_node_once_per_call() {
        let (graph, source_id, combine_id) = shared_source_graph();

        PreviewExecutor::new(true)
            .execute(&graph, combine_id, &context())
            .expect("should succeed");

        assert_eq!(calls_of(&graph, source_id), 1, "a shared source must be evaluated once per call, not once per consumer");
    }

    #[test]
    fn memoize_false_evaluates_a_shared_node_once_per_consumer() {
        let (graph, source_id, combine_id) = shared_source_graph();

        PreviewExecutor::new(false)
            .execute(&graph, combine_id, &context())
            .expect("should succeed");

        assert_eq!(calls_of(&graph, source_id), 2, "memoize:false must restore the original once-per-consumer behaviour");
    }

    #[test]
    fn neither_setting_persists_a_cache_across_separate_calls() {
        let mut graph = Graph::new(1, 1);
        let source_id = graph.add_node(Box::new(CountingSource { calls: Cell::new(0) }));

        let executor = PreviewExecutor::new(true);
        executor.execute(&graph, source_id, &context()).expect("should succeed");
        executor.execute(&graph, source_id, &context()).expect("should succeed");

        assert_eq!(calls_of(&graph, source_id), 2, "unlike RenderExecutor, PreviewExecutor must never persist a cache across separate execute() calls");
    }
}
