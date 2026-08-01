// src/compositor/graph/drive.rs

use crate::compositor::{input::Input, Context, Value};
use super::Graph;

impl Graph {
    /// Evaluate every PATCH node's mappings and push the results in -
    /// either into the wired SOURCE (target)'s own real parameter (a
    /// Number, or one channel of a Color, via its already-existing
    /// `set_parameter()`), or, when the target has no real parameter
    /// matching the mapped property at all, into PATCH's own internal
    /// per-channel state (its `execute()` then applies that as a raw
    /// pixel-channel substitution over the target's rendered image - see
    /// `operations::compose::patch`). Call once per tick, *before* the
    /// normal render/preview DAG walk - `RenderExecutor`'s own cache
    /// (which fingerprints a node via `get_parameter()`) picks up a
    /// target-parameter injection for free; PATCH itself is `is_live()`
    /// so its own re-execution is never skipped either.
    ///
    /// A flat, single pass over every node, not a recursive walk: the
    /// animation source referenced by REFERENCE is evaluated directly
    /// (`execute(ctx, &[])`, matching Phase A's zero-input animation
    /// ops), not through the normal Input-value resolution path -
    /// that's what makes a multi-output source's second-and-later output
    /// (Lissajous's Y, not just X) reachable at all.
    pub fn apply_patch_nodes(&mut self, ctx: &Context) {
        let patch_ids: Vec<super::NodeId> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                let node = slot.as_ref()?;
                if node.animation_mappings.is_empty() {
                    return None;
                }
                Some(super::NodeId { index: index as u32, generation: self.generations[index] })
            })
            .collect();

        for patch_id in patch_ids {
            let Some(patch_node) = self.resolve(patch_id) else { continue };
            let Some((_, target_id)) = patch_node.inputs.iter().find(|(key, _)| *key == Input::Source) else { continue };
            let Some((_, animation_id)) = patch_node.inputs.iter().find(|(key, _)| *key == Input::Reference) else { continue };
            let target_id = *target_id;
            let animation_id = *animation_id;
            let mappings = patch_node.animation_mappings.clone();

            let Some(animation_node) = self.resolve(animation_id) else { continue };
            let Ok(outputs) = animation_node.operation.execute(ctx, &[]) else { continue };

            for (property, output_index) in &mappings {
                let Some(value) = outputs.get(*output_index) else { continue };

                if let Some((base, channel)) = property.split_once('.') {
                    self.apply_colour_channel(target_id, base, channel, value);
                } else if !self.try_set_target_parameter(target_id, property, value) {
                    // Target has no real parameter by this name - it's
                    // one of the raw pixel-channel fallbacks
                    // (available_patch_properties only ever offers those
                    // when the target has no real parameters at all), so
                    // it belongs on PATCH's own internal state instead.
                    if let Some(patch_node) = self.resolve_mut(patch_id) {
                        let _ = patch_node.operation.set_parameter(property, value.clone());
                    }
                }
            }
        }
    }

    /// Try injecting `value` into `target`'s own parameter named `name`.
    /// Returns false (and touches nothing) if the target doesn't accept
    /// a parameter by that name - the caller's cue to fall back to
    /// PATCH's own raw-channel state instead.
    fn try_set_target_parameter(&mut self, target: super::NodeId, name: &str, value: &Value) -> bool {
        let Some(target_node) = self.resolve_mut(target) else { return false };
        target_node.operation.set_parameter(name, value.clone()).is_ok()
    }

    /// Read `base`'s current Color parameter off `target`, overwrite one
    /// channel, write it back - best-effort, silently does nothing if
    /// `base` isn't actually a Color parameter on `target` (shouldn't
    /// happen: `available_patch_properties` only ever offers a dotted
    /// property name for a parameter it already confirmed is Color-kind).
    fn apply_colour_channel(&mut self, target: super::NodeId, base: &str, channel: &str, value: &Value) {
        let Value::Number(n) = value else { return };

        let Some(target_node) = self.resolve(target) else { return };
        let Some(Value::Color(mut color)) = target_node.operation.get_parameter(base) else { return };

        match channel {
            "R" => color.r = *n as f32,
            "G" => color.g = *n as f32,
            "B" => color.b = *n as f32,
            "A" => color.a = *n as f32,
            _ => return,
        }

        if let Some(target_node) = self.resolve_mut(target) {
            let _ = target_node.operation.set_parameter(base, Value::Color(color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::metadata::{OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor, ParameterKind};
    use crate::compositor::{Operation, OperationDescriptor, OperationError};
    use crate::graphics::Color;
    use std::any::Any;
    use std::cell::Cell;

    fn descriptor(id: &'static str, label: &'static str) -> OperationDescriptor {
        OperationDescriptor { id, menu: "TEST", label, action: None, ui_action: None, create_node: None, submenu: None }
    }

    /// A stand-in for Lissajous: no inputs, two Number outputs.
    struct FakeAnimationSource {
        x: f64,
        y: f64,
    }

    impl Operation for FakeAnimationSource {
        fn descriptor(&self) -> OperationDescriptor { descriptor("fake_animation", "FAKE ANIMATION") }
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn metadata(&self) -> OperationMetadata {
            OperationMetadata { display_name: "FakeAnimation", category: OperationCategory::Animation, inputs: vec![], outputs: vec![OutputKind::Number, OutputKind::Number] }
        }
        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![Value::Number(self.x), Value::Number(self.y)])
        }
    }

    /// A stand-in for RING/RINGS: one real Number parameter and one
    /// real Color parameter, so both injection paths can be exercised.
    struct FakeParameterizedTarget {
        distance: Cell<f64>,
        key_color: Cell<Color>,
    }

    impl Operation for FakeParameterizedTarget {
        fn descriptor(&self) -> OperationDescriptor { descriptor("fake_target", "FAKE TARGET") }
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn metadata(&self) -> OperationMetadata {
            OperationMetadata { display_name: "FakeParameterizedTarget", category: OperationCategory::Generator, inputs: vec![], outputs: vec![] }
        }
        fn parameters(&self) -> Vec<ParameterDescriptor> {
            vec![
                ParameterDescriptor { name: "DISTANCE", kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None }, group: None },
                ParameterDescriptor { name: "KEY_COLOR", kind: ParameterKind::Color, group: None },
            ]
        }
        fn get_parameter(&self, name: &str) -> Option<Value> {
            match name {
                "DISTANCE" => Some(Value::Number(self.distance.get())),
                "KEY_COLOR" => Some(Value::Color(self.key_color.get())),
                _ => None,
            }
        }
        fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
            match (name, value) {
                ("DISTANCE", Value::Number(v)) => { self.distance.set(v); Ok(()) }
                ("KEY_COLOR", Value::Color(c)) => { self.key_color.set(c); Ok(()) }
                _ => Err(OperationError::UnknownParameter(name.to_string())),
            }
        }
        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> { Ok(vec![]) }
    }

    /// A stand-in for IMAGE 1: zero parameters at all, so
    /// available_patch_properties must fall back to raw R/G/B/A.
    struct FakeUnparameterizedTarget;

    impl Operation for FakeUnparameterizedTarget {
        fn descriptor(&self) -> OperationDescriptor { descriptor("fake_image", "FAKE IMAGE") }
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn metadata(&self) -> OperationMetadata {
            OperationMetadata { display_name: "FakeUnparameterizedTarget", category: OperationCategory::Source, inputs: vec![], outputs: vec![] }
        }
        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> { Ok(vec![]) }
    }

    fn patch_node() -> Box<dyn Operation> {
        Box::new(crate::operations::compose::Patch::new())
    }

    #[test]
    fn available_properties_lists_real_number_and_decomposed_colour_parameters() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(0.0), key_color: Cell::new(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();

        let properties = graph.available_patch_properties(patch);
        assert!(properties.contains(&"DISTANCE".to_string()));
        assert!(properties.contains(&"KEY_COLOR.R".to_string()));
        assert!(properties.contains(&"KEY_COLOR.A".to_string()));
    }

    #[test]
    fn available_properties_falls_back_to_raw_channels_when_target_has_none() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeUnparameterizedTarget));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();

        let properties = graph.available_patch_properties(patch);
        assert_eq!(properties, vec!["R", "G", "B", "A"]);
    }

    #[test]
    fn a_real_number_parameter_is_injected_into_the_target_itself() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(0.0), key_color: Cell::new(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 42.0, y: 7.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "DISTANCE", 0).unwrap();

        graph.apply_patch_nodes(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("DISTANCE") {
            Some(Value::Number(n)) => assert_eq!(n, 42.0),
            other => panic!("expected DISTANCE to be pushed to 42.0, got {:?}", other),
        }
    }

    #[test]
    fn a_colour_parameters_channel_is_injected_without_touching_the_others() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(0.0), key_color: Cell::new(Color { r: 0.1, g: 0.2, b: 0.3, a: 0.4 }) }));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 0.9, y: 0.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "KEY_COLOR.R", 0).unwrap();

        graph.apply_patch_nodes(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("KEY_COLOR") {
            Some(Value::Color(c)) => {
                assert!((c.r - 0.9).abs() < 1e-6, "R should be overwritten");
                assert!((c.g - 0.2).abs() < 1e-6, "G must be untouched");
            }
            other => panic!("expected a Color, got {:?}", other),
        }
    }

    #[test]
    fn a_raw_channel_lands_on_patch_itself_when_the_target_has_no_real_parameter() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeUnparameterizedTarget));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 0.5, y: 0.25 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "R", 0).unwrap();
        graph.set_patch_mapping(patch, "A", 1).unwrap();

        graph.apply_patch_nodes(&Context::default());

        let patch_op = graph.get_node(&patch).unwrap();
        match patch_op.get_parameter("R") {
            Some(Value::Number(n)) => assert_eq!(n, 0.5),
            other => panic!("expected R to land on PATCH itself, got {:?}", other),
        }
        match patch_op.get_parameter("A") {
            Some(Value::Number(n)) => assert_eq!(n, 0.25),
            other => panic!("expected A to land on PATCH itself, got {:?}", other),
        }
    }

    #[test]
    fn rewiring_source_clears_stale_mappings() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(0.0), key_color: Cell::new(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }));
        let other_target = graph.add_node(Box::new(FakeUnparameterizedTarget));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 1.0, y: 1.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "DISTANCE", 0).unwrap();

        graph.connect(patch, Input::Source, other_target).unwrap();

        let node = graph.resolve(patch).unwrap();
        assert!(node.animation_mappings.is_empty(), "rewiring SOURCE must drop mappings tied to the old target");
    }
}
