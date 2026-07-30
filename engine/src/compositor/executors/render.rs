// src/compositor/executors/render.rs

use std::collections::HashMap;

use crate::compositor::{Context, Input, OperationError, Value};
use crate::compositor::graph::{Graph, NodeId};
use super::Execute;
use crate::profiling::{measure_ms, Profile, ProfileEntry};

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

    pub fn execute_profiled(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<(Vec<Value>, Profile), OperationError> {
        let mut memo = HashMap::new();
        let mut entries = Vec::new();
        let (result, total_ms) =
            measure_ms(|| Self::evaluate_profiled(graph, node, ctx, &mut memo, &mut entries));
        let value = result?;
        Ok((vec![value], Profile { entries, total_ms }))
    }

    fn evaluate_profiled(
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
        memo: &mut HashMap<NodeId, Value>,
        entries: &mut Vec<ProfileEntry>,
    ) -> Result<Value, OperationError> {
        if let Some(cached) = memo.get(&node) {
            return Ok(cached.clone());
        }

        let node_data = graph.resolve(node).ok_or(OperationError::UnknownNode)?;

        let mut input_values: Vec<(Input, Value)> = Vec::new();

        for &(key, input_node_id) in &node_data.inputs {
            let value = Self::evaluate_profiled(graph, input_node_id, ctx, memo, entries)?;
            input_values.push((key, value));
        }

        let name = node_data.operation.metadata().display_name;
        let (outputs, duration_ms) = measure_ms(|| node_data.operation.execute(ctx, &input_values));
        let outputs = outputs?;
        entries.push(ProfileEntry { name, duration_ms });

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

    use crate::compositor::metadata::{OperationCategory, OperationMetadata};
    use crate::compositor::{OperationDescriptor, Operation};

    struct CountingSource {
        calls: Cell<u32>,
    }

    fn test_descriptor(id: &'static str, label: &'static str) -> OperationDescriptor {
        OperationDescriptor {
            id,
            menu: "TEST",
            label,
            action: None,
            ui_action: None,
            create_node: None,
        }
    }

    impl Operation for CountingSource {
        fn descriptor(&self) -> OperationDescriptor {
            test_descriptor("counting_source", "COUNTING SOURCE")
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
            Ok(vec![Value::Number(1.0)])
        }
    }

    struct Combine;

    impl Operation for Combine {
        fn descriptor(&self) -> OperationDescriptor {
            test_descriptor("combine", "COMBINE")
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
            Ok(vec![Value::Number(2.0)])
        }
    }

    #[test]
    fn a_node_feeding_two_consumers_is_evaluated_once_per_tick() {
        let mut graph = Graph::new(1, 1);

        let source_id = graph.add_node(Box::new(CountingSource { calls: Cell::new(0) }));

        let combine_id = graph.add_node(Box::new(Combine));
        graph.connect(combine_id, Input::Foreground, source_id).unwrap();
        graph.connect(combine_id, Input::Background, source_id).unwrap();

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

        let source_id = graph.add_node(Box::new(CountingSource { calls: Cell::new(0) }));

        let combine_id = graph.add_node(Box::new(Combine));
        graph.connect(combine_id, Input::Foreground, source_id).unwrap();
        graph.connect(combine_id, Input::Background, source_id).unwrap();

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

    #[test]
    fn execute_profiled_records_one_entry_per_evaluated_node_with_display_names() {
        let mut graph = Graph::new(1, 1);

        let source_id = graph.add_node(Box::new(CountingSource { calls: Cell::new(0) }));

        let combine_id = graph.add_node(Box::new(Combine));
        graph.connect(combine_id, Input::Foreground, source_id).unwrap();
        graph.connect(combine_id, Input::Background, source_id).unwrap();

        let ctx = Context::default();
        let (outputs, profile) = RenderExecutor
            .execute_profiled(&graph, combine_id, &ctx)
            .expect("should succeed");

        assert_eq!(outputs.len(), 1);

        // Shared source is memoized here too - one entry, not one per consumer.
        assert_eq!(profile.entries.len(), 2, "expected one entry for the source and one for Combine");
        assert_eq!(profile.entries[0].name, "CountingSource");
        assert_eq!(profile.entries[1].name, "Combine");
        assert!(profile.total_ms >= 0.0);

        let rendered = profile.to_string();
        assert!(rendered.contains("CountingSource:"));
        assert!(rendered.contains("Combine:"));
        assert!(rendered.contains("Total:"));
    }
}