// src/compositor/executors/preview.rs

use std::collections::HashMap;

use crate::compositor::{bbox::Rect, Context, Input, OperationError, Value};
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
            return Self::evaluate_unmemoized(graph, node, ctx).map(|(value, _bbox)| vec![value]);
        }

        let mut memo = HashMap::new();
        Self::evaluate_memoized(graph, node, ctx, &mut memo).map(|(value, _bbox)| vec![value])
    }
}

impl PreviewExecutor {
    /// Returns the node's own output value alongside the bbox it reported
    /// (see BBOX_CONVENTIONS.md) - threaded privately through this
    /// recursion only; the public `Execute::execute()` above still
    /// returns bare `Value`s.
    fn evaluate_unmemoized(
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<(Value, Rect), OperationError> {
        let node_data = graph.resolve(node).ok_or(OperationError::UnknownNode)?;

        let mut input_values: Vec<(Input, Value)> = Vec::new();
        let mut input_bboxes: Vec<(Input, Rect)> = Vec::new();

        for &(key, input_node_id) in &node_data.inputs {
            let (value, bbox) = Self::evaluate_unmemoized(graph, input_node_id, ctx)?;
            input_values.push((key, value));
            input_bboxes.push((key, bbox));
        }

        let node_ctx = Context { input_bboxes: input_bboxes.clone(), ..ctx.clone() };
        let outputs = node_data.operation.execute(&node_ctx, &input_values)?;
        // ok_or (not unwrap) so an operation that violates the "always
        // returns exactly one output" convention errors out this one
        // preview instead of panicking the whole WASM instance.
        let value = outputs.into_iter().next().ok_or(OperationError::NoOutput)?;
        let bbox = node_data.operation.output_bbox(&node_ctx, &input_bboxes, &value);

        Ok((value, bbox))
    }

    fn evaluate_memoized(
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
        memo: &mut HashMap<NodeId, (Value, Rect)>,
    ) -> Result<(Value, Rect), OperationError> {
        if let Some(cached) = memo.get(&node) {
            return Ok(cached.clone());
        }

        let node_data = graph.resolve(node).ok_or(OperationError::UnknownNode)?;

        let mut input_values: Vec<(Input, Value)> = Vec::new();
        let mut input_bboxes: Vec<(Input, Rect)> = Vec::new();

        for &(key, input_node_id) in &node_data.inputs {
            let (value, bbox) = Self::evaluate_memoized(graph, input_node_id, ctx, memo)?;
            input_values.push((key, value));
            input_bboxes.push((key, bbox));
        }

        let node_ctx = Context { input_bboxes: input_bboxes.clone(), ..ctx.clone() };
        let outputs = node_data.operation.execute(&node_ctx, &input_values)?;
        // See evaluate_unmemoized: ok_or, not unwrap, so a no-output
        // operation errors out this preview instead of panicking.
        let value = outputs.into_iter().next().ok_or(OperationError::NoOutput)?;
        let bbox = node_data.operation.output_bbox(&node_ctx, &input_bboxes, &value);

        memo.insert(node, (value.clone(), bbox));

        Ok((value, bbox))
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
                submenu: None,
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
                submenu: None,
            }
        }

        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "Combine",
                category: OperationCategory::Composite,
                inputs: vec![
                    crate::compositor::metadata::InputDescriptor { kind: Input::Foreground, accepts: &[] },
                    crate::compositor::metadata::InputDescriptor { kind: Input::Background, accepts: &[] },
                ],
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

    fn chromakey_add_pixels(executor: &PreviewExecutor) -> Vec<u8> {
        // Mirrors executors::render::tests's chromakey -> add pixel-identity
        // test, for PreviewExecutor's own (structurally identical) bbox
        // threading in evaluate_unmemoized/evaluate_memoized.
        use std::sync::Arc;
        use crate::graphics::{ImageFormat, U8Image};
        use crate::operations::compose::Add;
        use crate::operations::key::ChromaKey;
        use crate::operations::sources::ImageSource;

        let mut graph = Graph::new(1, 1);

        let mut green_source = ImageSource::new();
        green_source.set_image(Arc::new(U8Image {
            pixels: vec![0, 255, 0, 255],
            width: 1,
            height: 1,
            format: ImageFormat::Rgba8,
        }));
        let source_id = graph.add_node(Box::new(green_source));

        let chromakey_id = graph.add_node(Box::new(ChromaKey::new()));
        graph.connect(chromakey_id, Input::Source, source_id).unwrap();

        let mut backdrop = ImageSource::new();
        backdrop.set_image(Arc::new(U8Image {
            pixels: vec![10, 20, 30, 255],
            width: 1,
            height: 1,
            format: ImageFormat::Rgba8,
        }));
        let backdrop_id = graph.add_node(Box::new(backdrop));

        let add_id = graph.add_node(Box::new(Add::new()));
        graph.connect(add_id, Input::Foreground, chromakey_id).unwrap();
        graph.connect(add_id, Input::Background, backdrop_id).unwrap();

        let values = executor.execute(&graph, add_id, &context()).expect("should succeed");
        match &values[0] {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels.clone(),
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn phase_0_bbox_threading_does_not_change_a_real_multi_node_graphs_output_memoized() {
        let pixels = chromakey_add_pixels(&PreviewExecutor::new(true));
        assert_eq!(pixels, vec![10, 255, 30, 255], "PreviewExecutor's memoized path must be unchanged by Phase 0's threading");
    }

    #[test]
    fn phase_0_bbox_threading_does_not_change_a_real_multi_node_graphs_output_unmemoized() {
        let pixels = chromakey_add_pixels(&PreviewExecutor::new(false));
        assert_eq!(pixels, vec![10, 255, 30, 255], "PreviewExecutor's unmemoized path must be unchanged by Phase 0's threading");
    }
}
