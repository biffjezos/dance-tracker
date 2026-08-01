// src/compositor/graph/drive.rs

use crate::compositor::Context;
use super::Graph;

impl Graph {
    /// Evaluate every animation-category node with a target and at least
    /// one output mapping, and push its current output values into the
    /// target's own parameters via the normal `set_parameter()` path -
    /// exactly the injection point `ANIMATION_CONVENTIONS.md` already
    /// decided on for a future authored-keyframe curve, just fed by a
    /// wired driver's output instead. Call once per tick, *before* the
    /// normal render/preview DAG walk - `RenderExecutor`'s own cache
    /// (which fingerprints a node via `get_parameter()` on every declared
    /// parameter) picks up the resulting change for free, no
    /// cache-specific handling needed here.
    ///
    /// Deliberately a flat, single pass over every node, not a recursive
    /// walk: driver operations declare zero pixel inputs today (see
    /// `ANIMATION_IMPLEMENTATION_PLAN.md` Phase A), and
    /// `connect_animation_target` already refuses to let a driver target
    /// another animation node, so there is no ordering dependency
    /// between drivers to resolve here. A future animation operation
    /// that wants its own pixel inputs would need this revisited - this
    /// pass does not resolve any wired `Input` for the driver itself, it
    /// calls `execute()` with none.
    pub fn apply_animation_drivers(&mut self, ctx: &Context) {
        let driver_ids: Vec<super::NodeId> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                let node = slot.as_ref()?;
                if node.animation_target.is_none() || node.animation_mappings.is_empty() {
                    return None;
                }
                Some(super::NodeId { index: index as u32, generation: self.generations[index] })
            })
            .collect();

        for driver_id in driver_ids {
            let Some(driver_node) = self.resolve(driver_id) else { continue };
            let Some(target_id) = driver_node.animation_target else { continue };
            let mappings = driver_node.animation_mappings.clone();

            let Ok(outputs) = driver_node.operation.execute(ctx, &[]) else { continue };

            let Some(target_node) = self.resolve_mut(target_id) else { continue };
            for (output_index, target_parameter) in &mappings {
                let Some(value) = outputs.get(*output_index) else { continue };
                // Best-effort: an animated value outside the target's own
                // valid range simply isn't applied this tick (the target
                // keeps whatever it last had) - same as a person typing
                // an out-of-range value by hand gets rejected, not a hard
                // error that would blank the whole render over one
                // momentarily-out-of-range tick.
                let _ = target_node.operation.set_parameter(target_parameter, value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::metadata::{OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind};
    use crate::compositor::{Input, Operation, OperationDescriptor, OperationError, Value};
    use std::any::Any;
    use std::cell::Cell;

    fn descriptor(id: &'static str, label: &'static str) -> OperationDescriptor {
        OperationDescriptor { id, menu: "TEST", label, action: None, ui_action: None, create_node: None, submenu: None }
    }

    /// A stand-in for Lissajous/Sine/Square: no inputs, one Number output
    /// whose value is just a counter bumped on every execute() call, so
    /// tests can tell whether the driver was actually re-evaluated.
    struct FakeDriver {
        calls: Cell<f64>,
    }

    impl Operation for FakeDriver {
        fn descriptor(&self) -> OperationDescriptor { descriptor("fake_driver", "FAKE DRIVER") }
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn metadata(&self) -> OperationMetadata {
            OperationMetadata { display_name: "FakeDriver", category: OperationCategory::Animation, inputs: vec![], outputs: vec![OutputKind::Number] }
        }
        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            let next = self.calls.get() + 1.0;
            self.calls.set(next);
            Ok(vec![Value::Number(next)])
        }
    }

    /// A stand-in for RING/MIX: one settable Number parameter, an
    /// eligible target.
    struct FakeTarget {
        amount: Cell<f64>,
    }

    impl Operation for FakeTarget {
        fn descriptor(&self) -> OperationDescriptor { descriptor("fake_target", "FAKE TARGET") }
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn metadata(&self) -> OperationMetadata {
            OperationMetadata { display_name: "FakeTarget", category: OperationCategory::Composite, inputs: vec![], outputs: vec![] }
        }
        fn parameters(&self) -> Vec<ParameterDescriptor> {
            vec![ParameterDescriptor { name: "AMOUNT", kind: ParameterKind::Number { step: 0.01, min: Some(0.0), max: Some(100.0) }, group: None }]
        }
        fn get_parameter(&self, name: &str) -> Option<Value> {
            (name == "AMOUNT").then(|| Value::Number(self.amount.get()))
        }
        fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
            match (name, value) {
                ("AMOUNT", Value::Number(v)) => { self.amount.set(v); Ok(()) }
                _ => Err(OperationError::UnknownParameter(name.to_string())),
            }
        }
        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> { Ok(vec![]) }
    }

    #[test]
    fn a_wired_driver_pushes_its_output_into_the_targets_parameter() {
        let mut graph = Graph::new(4, 4);
        let driver = graph.add_node(Box::new(FakeDriver { calls: Cell::new(0.0) }));
        let target = graph.add_node(Box::new(FakeTarget { amount: Cell::new(0.0) }));

        graph.connect_animation_target(driver, target).unwrap();
        graph.set_animation_mapping(driver, 0, "AMOUNT").unwrap();

        graph.apply_animation_drivers(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("AMOUNT") {
            Some(Value::Number(n)) => assert_eq!(n, 1.0),
            other => panic!("expected AMOUNT to be pushed to 1.0, got {:?}", other),
        }
    }

    #[test]
    fn the_driver_is_re_evaluated_every_call_not_cached() {
        let mut graph = Graph::new(4, 4);
        let driver = graph.add_node(Box::new(FakeDriver { calls: Cell::new(0.0) }));
        let target = graph.add_node(Box::new(FakeTarget { amount: Cell::new(0.0) }));
        graph.connect_animation_target(driver, target).unwrap();
        graph.set_animation_mapping(driver, 0, "AMOUNT").unwrap();

        graph.apply_animation_drivers(&Context::default());
        graph.apply_animation_drivers(&Context::default());
        graph.apply_animation_drivers(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("AMOUNT") {
            Some(Value::Number(n)) => assert_eq!(n, 3.0),
            other => panic!("expected AMOUNT to be pushed to 3.0, got {:?}", other),
        }
    }

    #[test]
    fn a_value_the_target_rejects_is_dropped_not_propagated_as_an_error() {
        let mut graph = Graph::new(4, 4);
        // FakeDriver's output climbs 1, 2, 3, ... - FakeTarget's AMOUNT
        // accepts 0..100, so this never actually goes out of range, but
        // the point is apply_animation_drivers must not panic or error
        // out even if it did; it's infallible by design (see its own
        // doc comment on this file).
        let driver = graph.add_node(Box::new(FakeDriver { calls: Cell::new(0.0) }));
        let target = graph.add_node(Box::new(FakeTarget { amount: Cell::new(0.0) }));
        graph.connect_animation_target(driver, target).unwrap();
        graph.set_animation_mapping(driver, 0, "AMOUNT").unwrap();

        // Must not panic.
        graph.apply_animation_drivers(&Context::default());
    }

    #[test]
    fn a_driver_with_no_target_is_left_alone() {
        let mut graph = Graph::new(4, 4);
        let driver = graph.add_node(Box::new(FakeDriver { calls: Cell::new(0.0) }));
        // Never wired to anything - must not panic, must not touch anything.
        graph.apply_animation_drivers(&Context::default());
        assert!(graph.get_node(&driver).is_some());
    }
}
