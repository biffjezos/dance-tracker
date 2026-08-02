// src/compositor/executors/render.rs

use std::cell::RefCell;
use std::collections::HashMap;

use crate::compositor::{bbox::Rect, Context, Input, Operation, OperationError, Value};
use crate::compositor::value::{value_ptr_eq, value_to_text};
use crate::compositor::graph::{Graph, NodeId};
use super::Execute;
use crate::profiling::{measure_ms, Profile, ProfileEntry};

/// What a node produced last tick, and everything that could make that
/// stale: its own parameter values, and the exact resolved input values
/// that fed it. If both are unchanged this tick, re-running execute()
/// would only reproduce the same result - so it's skipped.
struct CachedNode {
    param_fingerprint: Vec<(String, String)>,
    inputs: Vec<(Input, Value)>,
    value: Value,
    bbox: Rect,
}

/// Evaluates a graph the same way every tick, reusing last tick's result
/// for any node whose own state and resolved inputs haven't changed. This
/// only helps across separate `execute()` calls on the *same* instance -
/// the app keeps one persistent RenderExecutor for exactly that reason.
#[derive(Default)]
pub struct RenderExecutor {
    cache: RefCell<HashMap<NodeId, CachedNode>>,
}

impl RenderExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop a node's cached result - call this when the node is removed
    /// from the graph, so the cache doesn't hold a stale entry forever.
    pub fn invalidate(&self, node: NodeId) {
        self.cache.borrow_mut().remove(&node);
    }

    fn fingerprint(operation: &dyn Operation) -> Vec<(String, String)> {
        operation
            .parameters()
            .into_iter()
            .map(|descriptor| {
                let value = operation
                    .get_parameter(descriptor.name)
                    .map(|v| value_to_text(&v))
                    .unwrap_or_default();
                (descriptor.name.to_string(), value)
            })
            .collect()
    }
}

impl Execute for RenderExecutor {
    fn execute(
        &self,
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
    ) -> Result<Vec<Value>, OperationError> {
        let mut memo = HashMap::new();
        let (value, _bbox) = self.evaluate(graph, node, ctx, &mut memo)?;
        Ok(vec![value])
    }
}

impl RenderExecutor {
    /// Returns the node's own output value alongside the bbox it reported
    /// for that output (see BBOX_CONVENTIONS.md) - threaded privately
    /// through this recursion only; the public `Execute::execute()` above
    /// still returns bare `Value`s.
    fn evaluate(
        &self,
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
            let (value, bbox) = self.evaluate(graph, input_node_id, ctx, memo)?;
            input_values.push((key, value));
            input_bboxes.push((key, bbox));
        }

        let param_fingerprint = Self::fingerprint(node_data.operation.as_ref());
        let live = node_data.operation.is_live();

        if !live {
            if let Some(cached) = self.cache.borrow().get(&node) {
                let inputs_match = cached.inputs.len() == input_values.len()
                    && cached
                        .inputs
                        .iter()
                        .zip(&input_values)
                        .all(|((ck, cv), (k, v))| ck == k && value_ptr_eq(cv, v));

                if inputs_match && cached.param_fingerprint == param_fingerprint {
                    let result = (cached.value.clone(), cached.bbox);
                    memo.insert(node, result.clone());
                    return Ok(result);
                }
            }
        }

        // The per-node Context: a clone of the base ctx with input_bboxes
        // overridden to this node's own resolved inputs, built immediately
        // before execute() - see BBOX_CONVENTIONS.md on why this lives on
        // Context rather than as a new execute() parameter.
        let node_ctx = Context { input_bboxes: input_bboxes.clone(), ..ctx.clone() };

        let outputs = node_data.operation.execute(&node_ctx, &input_values)?;
        // ok_or (not unwrap) so a no-output operation errors out this tick
        // instead of panicking the whole render loop.
        let value = outputs.into_iter().next().ok_or(OperationError::NoOutput)?;
        let bbox = node_data.operation.output_bbox(&node_ctx, &input_bboxes, &value);

        if !live {
            self.cache.borrow_mut().insert(node, CachedNode {
                param_fingerprint,
                inputs: input_values,
                value: value.clone(),
                bbox,
            });
        }

        memo.insert(node, (value.clone(), bbox));

        Ok((value, bbox))
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
        let (value, _bbox) = result?;
        Ok((vec![value], Profile { entries, total_ms }))
    }

    fn evaluate_profiled(
        graph: &Graph,
        node: NodeId,
        ctx: &Context,
        memo: &mut HashMap<NodeId, (Value, Rect)>,
        entries: &mut Vec<ProfileEntry>,
    ) -> Result<(Value, Rect), OperationError> {
        if let Some(cached) = memo.get(&node) {
            return Ok(cached.clone());
        }

        let node_data = graph.resolve(node).ok_or(OperationError::UnknownNode)?;

        let mut input_values: Vec<(Input, Value)> = Vec::new();
        let mut input_bboxes: Vec<(Input, Rect)> = Vec::new();

        for &(key, input_node_id) in &node_data.inputs {
            let (value, bbox) = Self::evaluate_profiled(graph, input_node_id, ctx, memo, entries)?;
            input_values.push((key, value));
            input_bboxes.push((key, bbox));
        }

        let name = node_data.operation.metadata().display_name;
        let node_ctx = Context { input_bboxes: input_bboxes.clone(), ..ctx.clone() };
        crate::graphics::mask::reset_pixels_computed();
        let (outputs, duration_ms) = measure_ms(|| node_data.operation.execute(&node_ctx, &input_values));
        let pixels_computed = crate::graphics::mask::take_pixels_computed();
        let outputs = outputs?;
        entries.push(ProfileEntry { name, duration_ms, pixels_computed });

        // ok_or (not unwrap): see evaluate() above.
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
            submenu: None,
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
                inputs: vec![
                    crate::compositor::metadata::InputDescriptor { kind: Input::Foreground, accepts: &[] },
                    crate::compositor::metadata::InputDescriptor { kind: Input::Background, accepts: &[] },
                ],
                outputs: vec![],
            }
        }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![Value::Number(2.0)])
        }
    }

    /// A source with one settable parameter, so tests can force a change
    /// in a node's own state (as opposed to its wired inputs) between ticks.
    struct ConfigurableSource {
        amount: Cell<f64>,
        calls: Cell<u32>,
    }

    impl Operation for ConfigurableSource {
        fn descriptor(&self) -> OperationDescriptor {
            test_descriptor("configurable_source", "CONFIGURABLE SOURCE")
        }

        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "ConfigurableSource",
                category: OperationCategory::Source,
                inputs: vec![],
                outputs: vec![],
            }
        }

        fn parameters(&self) -> Vec<crate::compositor::metadata::ParameterDescriptor> {
            vec![crate::compositor::metadata::ParameterDescriptor {
                name: "AMOUNT",
                kind: crate::compositor::metadata::ParameterKind::Number {
                    step: 1.0,
                    min: None,
                    max: None,
                },
                group: None,
            }]
        }

        fn get_parameter(&self, name: &str) -> Option<Value> {
            match name {
                "AMOUNT" => Some(Value::Number(self.amount.get())),
                _ => None,
            }
        }

        fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
            match (name, value) {
                ("AMOUNT", Value::Number(v)) => {
                    self.amount.set(v);
                    Ok(())
                }
                _ => Err(OperationError::UnknownParameter(name.to_string())),
            }
        }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            self.calls.set(self.calls.get() + 1);
            Ok(vec![Value::Number(self.amount.get())])
        }
    }

    #[test]
    fn a_node_with_unchanged_state_and_inputs_reuses_last_ticks_result() {
        let mut graph = Graph::new(1, 1);

        let source_id = graph.add_node(Box::new(CountingSource { calls: Cell::new(0) }));
        let combine_id = graph.add_node(Box::new(Combine));
        graph.connect(combine_id, Input::Foreground, source_id).unwrap();
        graph.connect(combine_id, Input::Background, source_id).unwrap();

        let ctx = Context::default();
        let executor = RenderExecutor::new();
        executor.execute(&graph, combine_id, &ctx).expect("should succeed");
        executor.execute(&graph, combine_id, &ctx).expect("should succeed");

        let calls = graph
            .resolve(source_id)
            .unwrap()
            .operation
            .as_any()
            .downcast_ref::<CountingSource>()
            .unwrap()
            .calls
            .get();

        assert_eq!(calls, 1, "nothing changed - the second tick should reuse the cached result");
    }

    #[test]
    fn changing_a_parameter_forces_re_execution_next_tick() {
        let mut graph = Graph::new(1, 1);
        let source_id = graph.add_node(Box::new(ConfigurableSource {
            amount: Cell::new(1.0),
            calls: Cell::new(0),
        }));

        let ctx = Context::default();
        let executor = RenderExecutor::new();
        executor.execute(&graph, source_id, &ctx).expect("should succeed");
        executor.execute(&graph, source_id, &ctx).expect("should succeed");

        graph
            .get_node_mut(&source_id)
            .unwrap()
            .set_parameter("AMOUNT", Value::Number(2.0))
            .unwrap();
        executor.execute(&graph, source_id, &ctx).expect("should succeed");

        let calls = graph
            .resolve(source_id)
            .unwrap()
            .operation
            .as_any()
            .downcast_ref::<ConfigurableSource>()
            .unwrap()
            .calls
            .get();

        assert_eq!(
            calls, 2,
            "the unchanged second tick should be cached, but changing AMOUNT must force a third real execution"
        );
    }

    #[test]
    fn rewiring_an_input_forces_re_execution_next_tick() {
        let mut graph = Graph::new(1, 1);

        let a = graph.add_node(Box::new(CountingSource { calls: Cell::new(0) }));
        let b = graph.add_node(Box::new(CountingSource { calls: Cell::new(0) }));
        let combine_id = graph.add_node(Box::new(Combine));
        graph.connect(combine_id, Input::Foreground, a).unwrap();
        graph.connect(combine_id, Input::Background, a).unwrap();

        let ctx = Context::default();
        let executor = RenderExecutor::new();
        executor.execute(&graph, combine_id, &ctx).expect("should succeed");
        executor.execute(&graph, combine_id, &ctx).expect("should succeed");

        // Rewire Background from a to b - Combine's own cache must not
        // survive this even though Combine itself has no parameters.
        graph.connect(combine_id, Input::Background, b).unwrap();
        executor.execute(&graph, combine_id, &ctx).expect("should succeed");

        let calls_of = |node: NodeId| {
            graph
                .resolve(node)
                .unwrap()
                .operation
                .as_any()
                .downcast_ref::<CountingSource>()
                .unwrap()
                .calls
                .get()
        };

        assert_eq!(calls_of(b), 1, "b was just wired in and must be evaluated");
        assert_eq!(
            calls_of(a), 1,
            "a is still wired to Foreground unchanged, so its cached result must survive \
             the unrelated Background rewire, not just get invalidated wholesale"
        );
    }

    #[test]
    fn a_node_feeding_two_consumers_is_evaluated_once_per_tick() {
        let mut graph = Graph::new(1, 1);

        let source_id = graph.add_node(Box::new(CountingSource { calls: Cell::new(0) }));

        let combine_id = graph.add_node(Box::new(Combine));
        graph.connect(combine_id, Input::Foreground, source_id).unwrap();
        graph.connect(combine_id, Input::Background, source_id).unwrap();

        let ctx = Context::default();
        RenderExecutor::new().execute(&graph, combine_id, &ctx).expect("should succeed");

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
        RenderExecutor::new().execute(&graph, combine_id, &ctx).expect("should succeed");
        RenderExecutor::new().execute(&graph, combine_id, &ctx).expect("should succeed");

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
        let (outputs, profile) = RenderExecutor::new()
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

    /// Stands in for CameraSource/VideoSource: zero inputs, zero parameters
    /// (so its fingerprint never changes), but is_live() opts it out of the
    /// cross-tick cache anyway - the same shape a live camera/video stream
    /// has, which is exactly what made LIVE OUTPUT freeze on its first
    /// captured frame before is_live() existed.
    struct LiveCountingSource {
        calls: Cell<u32>,
    }

    impl Operation for LiveCountingSource {
        fn descriptor(&self) -> OperationDescriptor {
            test_descriptor("live_counting_source", "LIVE COUNTING SOURCE")
        }

        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "LiveCountingSource",
                category: OperationCategory::Source,
                inputs: vec![],
                outputs: vec![],
            }
        }

        fn is_live(&self) -> bool {
            true
        }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            self.calls.set(self.calls.get() + 1);
            Ok(vec![Value::Number(self.calls.get() as f64)])
        }
    }

    #[test]
    fn a_live_node_is_re_executed_every_tick_despite_unchanged_state_and_inputs() {
        let mut graph = Graph::new(1, 1);
        let source_id = graph.add_node(Box::new(LiveCountingSource { calls: Cell::new(0) }));

        let ctx = Context::default();
        let executor = RenderExecutor::new();
        executor.execute(&graph, source_id, &ctx).expect("should succeed");
        executor.execute(&graph, source_id, &ctx).expect("should succeed");
        executor.execute(&graph, source_id, &ctx).expect("should succeed");

        let calls = graph
            .resolve(source_id)
            .unwrap()
            .operation
            .as_any()
            .downcast_ref::<LiveCountingSource>()
            .unwrap()
            .calls
            .get();

        assert_eq!(calls, 3, "a live source must never be served from the cross-tick cache");
    }

    #[test]
    fn phase_0_bbox_threading_does_not_change_a_real_multi_node_graphs_output() {
        // A real chromakey -> add chain, run through the actual graph and
        // RenderExecutor - proves Phase 0's (Value, Rect) threading and
        // per-node Context construction changed nothing observable. Every
        // operation here still only overrides the default (full-frame)
        // output_bbox, so this is exactly today's pixel behavior.
        use std::sync::Arc;
        use crate::compositor::graph::Graph;
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

        let ctx = Context {
            meta: crate::compositor::Meta { width: 1, height: 1, ..Default::default() },
            ..Default::default()
        };

        let values = RenderExecutor::new().execute(&graph, add_id, &ctx).expect("should succeed");
        let pixels = match &values[0] {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels.clone(),
            other => panic!("expected a float image, got {:?}", other),
        };

        // Pure green is keyed to alpha=0 by chromakey's own default
        // threshold, but its RGB (0,1,0) still passes through unchanged -
        // ADD sums all 4 channels uniformly regardless of alpha (see
        // add.rs's own doc comment), so the result is the backdrop's
        // colour plus the still-present green RGB, clamped: R=10/255+0,
        // G=20/255+1.0 (clamped to 1.0), B=30/255+0, A=1.0+0.
        assert_eq!(pixels, vec![10, 255, 30, 255], "the graph's real output must be unchanged by Phase 0's threading");
    }

    #[test]
    fn a_live_sources_consumer_also_re_evaluates_every_tick() {
        let mut graph = Graph::new(1, 1);

        let source_id = graph.add_node(Box::new(LiveCountingSource { calls: Cell::new(0) }));
        let combine_id = graph.add_node(Box::new(Combine));
        graph.connect(combine_id, Input::Foreground, source_id).unwrap();
        graph.connect(combine_id, Input::Background, source_id).unwrap();

        let ctx = Context::default();
        let executor = RenderExecutor::new();
        executor.execute(&graph, combine_id, &ctx).expect("should succeed");
        executor.execute(&graph, combine_id, &ctx).expect("should succeed");

        let calls = graph
            .resolve(source_id)
            .unwrap()
            .operation
            .as_any()
            .downcast_ref::<LiveCountingSource>()
            .unwrap()
            .calls
            .get();

        assert_eq!(
            calls, 2,
            "the live source's output value differs each tick, so its non-live consumer \
             must see a resolved-input mismatch (value_ptr_eq) and re-execute too, not \
             just the source itself"
        );
    }
}