// src/compositor/graph/drive.rs

use crate::compositor::{input::Input, metadata::ParameterKind, Context, Value};
use super::{Graph, node::PatchMode};

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
    ///
    /// Every plain (non-dotted) property mapping across *every* PATCH
    /// node is applied before any dotted Color-channel mapping on any
    /// PATCH node ("RING_SELECTOR" before "RING_COLOR.R") - two full
    /// passes over `patch_ids`, not one. A selector-shaped Number
    /// parameter (RING_SELECTOR, grouped with a Color parameter - see
    /// `available_patch_properties`) needs to land *this* tick before a
    /// same-tick Color-channel write that depends on it
    /// (`apply_colour_channel` reads whatever the target's selector
    /// currently points at), and that selector and that colour write can
    /// legitimately live on two *different* PATCH nodes (driven by two
    /// different animation sources, e.g. a step function selecting the
    /// ring and a sine driving its colour) - so this can't just sort one
    /// PATCH node's own mappings and call it done; it has to hold
    /// regardless of which PATCH node each mapping lives on or which
    /// order the nodes were created in.
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

        for dotted in [false, true] {
            for &patch_id in &patch_ids {
                let Some(patch_node) = self.resolve(patch_id) else { continue };
                let Some((_, target_id)) = patch_node.inputs.iter().find(|(key, _)| *key == Input::Source) else { continue };
                let Some((_, animation_id)) = patch_node.inputs.iter().find(|(key, _)| *key == Input::Reference) else { continue };
                let target_id = *target_id;
                let animation_id = *animation_id;
                let mappings: Vec<_> = patch_node.animation_mappings.iter()
                    .filter(|mapping| mapping.property.contains('.') == dotted)
                    .cloned()
                    .collect();
                if mappings.is_empty() { continue; }

                let Some(animation_node) = self.resolve(animation_id) else { continue };
                let Ok(outputs) = animation_node.operation.execute(ctx, &[]) else { continue };

                for mapping in &mappings {
                    let Some(raw_value) = outputs.get(mapping.output_index) else { continue };
                    let value = Self::combine_patch_value(mapping.mode, mapping.base, raw_value);

                    if let Some((base, channel)) = mapping.property.split_once('.') {
                        self.apply_colour_channel(target_id, base, channel, &value);
                    } else if !self.try_set_target_parameter(target_id, &mapping.property, &value) {
                        // Target has no real parameter by this name - it's
                        // one of the raw pixel-channel fallbacks
                        // (available_patch_properties only ever offers those
                        // when the target has no real parameters at all), so
                        // it belongs on PATCH's own internal state instead.
                        if let Some(patch_node) = self.resolve_mut(patch_id) {
                            let _ = patch_node.operation.set_parameter(&mapping.property, value.clone());
                        }
                    }
                }
            }
        }
    }

    /// Combine a mapping's mode with its captured `base` and the
    /// animation source's raw output value for this tick. Replace passes
    /// the raw value straight through (today's original, only-ever
    /// behaviour); Add/Subtract offset from `base` instead of the raw
    /// value replacing the property outright - see `PatchMapping::base`.
    /// Non-Number values (nothing produces these today - every animation
    /// source's outputs are Number - but nothing here assumes otherwise)
    /// pass through unchanged regardless of mode, since Add/Subtract only
    /// mean something for a scalar.
    fn combine_patch_value(mode: PatchMode, base: f64, value: &Value) -> Value {
        match (mode, value) {
            (PatchMode::Add, Value::Number(n)) => Value::Number(base + n),
            (PatchMode::Subtract, Value::Number(n)) => Value::Number(base - n),
            (_, v) => v.clone(),
        }
    }

    /// Try injecting `value` into `target`'s own parameter named `name`.
    /// Returns false (and touches nothing) if the target doesn't accept
    /// a parameter by that name - the caller's cue to fall back to
    /// PATCH's own raw-channel state instead.
    ///
    /// A Number value is clamped to that parameter's own declared min/max
    /// first, same as a manual stepper edit already is (nodeEditContexts.js's
    /// renderNumberParameter) - an animation source's raw range rarely lines
    /// up with a target's own (Lissajous's X/Y swing -AMPLITUDE..AMPLITUDE,
    /// but e.g. RADIUS only ever accepts >=0). Without clamping here, every
    /// tick the animation value spends outside the target's range would
    /// silently fail set_parameter and leave the property stuck at its last
    /// in-range value instead of tracking the animation, rather than
    /// pinning to the nearest value the target actually accepts.
    pub(super) fn try_set_target_parameter(&mut self, target: super::NodeId, name: &str, value: &Value) -> bool {
        let Some(target_node) = self.resolve(target) else { return false };
        let value = match value {
            Value::Number(n) => {
                let bounds = target_node.operation.parameters().into_iter().find(|p| p.name == name).map(|p| p.kind);
                let mut n = *n;
                if let Some(ParameterKind::Number { min, max, .. }) = bounds {
                    if let Some(min) = min { n = n.max(min); }
                    if let Some(max) = max { n = n.min(max); }
                }
                Value::Number(n)
            }
            other => other.clone(),
        };

        let Some(target_node) = self.resolve_mut(target) else { return false };
        target_node.operation.set_parameter(name, value).is_ok()
    }

    /// Read `base`'s current Color parameter off `target`, overwrite one
    /// channel, write it back - best-effort, silently does nothing if
    /// `base` isn't actually a Color parameter on `target` (shouldn't
    /// happen: `available_patch_properties` only ever offers a dotted
    /// property name for a parameter it already confirmed is Color-kind).
    pub(super) fn apply_colour_channel(&mut self, target: super::NodeId, base: &str, channel: &str, value: &Value) {
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

    /// A stand-in for RING specifically: a Number parameter (SELECTOR)
    /// grouped alongside a Color parameter (SLOT_COLOR) - the same shape
    /// as RING_SELECTOR + RING_COLOR under RING's "COLOUR" group: SELECTOR
    /// picks which of 3 colour slots SLOT_COLOR reads/writes, mirroring
    /// RING's own selected_ring/colors relationship closely enough to
    /// exercise the same-tick selector-before-colour-write ordering.
    struct FakeSelectorTarget {
        selector: Cell<usize>, // 1-based, 1..=3
        slots: [Cell<Color>; 3],
    }

    impl FakeSelectorTarget {
        fn new() -> Self {
            let white = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
            Self { selector: Cell::new(1), slots: [Cell::new(white), Cell::new(white), Cell::new(white)] }
        }
    }

    impl Operation for FakeSelectorTarget {
        fn descriptor(&self) -> OperationDescriptor { descriptor("fake_selector_target", "FAKE SELECTOR TARGET") }
        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn metadata(&self) -> OperationMetadata {
            OperationMetadata { display_name: "FakeSelectorTarget", category: OperationCategory::Generator, inputs: vec![], outputs: vec![] }
        }
        fn parameters(&self) -> Vec<ParameterDescriptor> {
            vec![
                ParameterDescriptor { name: "SELECTOR", kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: Some(3.0) }, group: Some("COLOUR") },
                ParameterDescriptor { name: "SLOT_COLOR", kind: ParameterKind::Color, group: Some("COLOUR") },
            ]
        }
        fn get_parameter(&self, name: &str) -> Option<Value> {
            match name {
                "SELECTOR" => Some(Value::Number(self.selector.get() as f64)),
                "SLOT_COLOR" => Some(Value::Color(self.slots[self.selector.get() - 1].get())),
                _ => None,
            }
        }
        fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
            match (name, value) {
                ("SELECTOR", Value::Number(v)) => {
                    let index = v.round() as i64;
                    if !(1..=3).contains(&index) {
                        return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                    }
                    self.selector.set(index as usize);
                    Ok(())
                }
                ("SLOT_COLOR", Value::Color(c)) => {
                    self.slots[self.selector.get() - 1].set(c);
                    Ok(())
                }
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
    fn available_properties_includes_a_number_grouped_with_a_colour_parameter() {
        // RING_SELECTOR's exact shape: a Number sharing a group with a
        // Color has no rendering effect on its own, but mapped *together*
        // with that Color in the same PATCH, it's how "select ring N,
        // recolour it" works - so it must still be offered, not hidden.
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeSelectorTarget::new()));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();

        let properties = graph.available_patch_properties(patch);
        assert!(properties.contains(&"SELECTOR".to_string()));
        assert!(properties.contains(&"SLOT_COLOR.R".to_string()));
    }

    #[test]
    fn a_selector_mapping_lands_before_a_colour_channel_mapping_in_the_same_tick() {
        // The exact reported workflow: SELECTOR picks which of 3 colour
        // slots gets recoloured, both driven by the same animation in the
        // same tick. Mapped here in the "wrong" order on purpose (colour
        // channel before selector) to prove apply_patch_nodes's own
        // ordering fix, not insertion order, decides which lands first -
        // without it, this colour write would land on slot 1 (last
        // tick's/the initial selection) instead of slot 2 (this tick's).
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeSelectorTarget::new()));
        // Outputs: index 0 -> the new SELECTOR (2), index 1 -> the new R (0.75).
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 2.0, y: 0.75 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "SLOT_COLOR.R", 1, PatchMode::Replace).unwrap();
        graph.set_patch_mapping(patch, "SELECTOR", 0, PatchMode::Replace).unwrap();

        graph.apply_patch_nodes(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("SELECTOR") {
            Some(Value::Number(n)) => assert_eq!(n, 2.0),
            other => panic!("expected SELECTOR to be 2.0, got {:?}", other),
        }
        // SELECTOR is now 2, so SLOT_COLOR reads slot 2 - the R write
        // must have landed there, not on slot 1.
        match target_op.get_parameter("SLOT_COLOR") {
            Some(Value::Color(c)) => assert!((c.r - 0.75).abs() < 1e-6, "expected slot 2's R to be overwritten to 0.75, got {:?}", c),
            other => panic!("expected a Color, got {:?}", other),
        }
    }

    #[test]
    fn ordering_holds_across_two_separate_patch_nodes_targeting_the_same_source() {
        // The selector and the colour write can legitimately live on two
        // *different* PATCH nodes (a step function driving the selector,
        // a separate animation driving the colour). Created here in the
        // "wrong" order on purpose - the colour PATCH first, the selector
        // PATCH second - to prove the fix isn't just "sort within one
        // node's own mappings" but holds regardless of which PATCH node
        // (or graph creation order) each mapping lives on.
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeSelectorTarget::new()));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 2.0, y: 0.75 }));

        let colour_patch = graph.add_node(patch_node());
        graph.connect(colour_patch, Input::Source, target).unwrap();
        graph.connect(colour_patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(colour_patch, "SLOT_COLOR.R", 1, PatchMode::Replace).unwrap();

        let selector_patch = graph.add_node(patch_node());
        graph.connect(selector_patch, Input::Source, target).unwrap();
        graph.connect(selector_patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(selector_patch, "SELECTOR", 0, PatchMode::Replace).unwrap();

        graph.apply_patch_nodes(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("SLOT_COLOR") {
            Some(Value::Color(c)) => assert!((c.r - 0.75).abs() < 1e-6, "expected slot 2's (this tick's selection) R to be overwritten, got {:?}", c),
            other => panic!("expected a Color, got {:?}", other),
        }
    }

    #[test]
    fn a_real_number_parameter_is_injected_into_the_target_itself() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(0.0), key_color: Cell::new(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 42.0, y: 7.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "DISTANCE", 0, PatchMode::Replace).unwrap();

        graph.apply_patch_nodes(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("DISTANCE") {
            Some(Value::Number(n)) => assert_eq!(n, 42.0),
            other => panic!("expected DISTANCE to be pushed to 42.0, got {:?}", other),
        }
    }

    #[test]
    fn an_out_of_range_number_clamps_to_the_targets_own_bounds_instead_of_freezing() {
        // Regression: FakeParameterizedTarget's DISTANCE only accepts >=0
        // (min: Some(0.0)), same shape as RADIUS/DISTANCE/GHOST_COUNT on
        // real operations. An animation source's raw range (Lissajous's
        // X/Y swing -AMPLITUDE..AMPLITUDE by default) routinely dips
        // negative - before this fix, set_parameter would reject the
        // out-of-range value outright and DISTANCE would just stay stuck
        // at its previous value instead of tracking the animation at all
        // whenever it went negative.
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(3.0), key_color: Cell::new(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: -5.0, y: 0.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "DISTANCE", 0, PatchMode::Replace).unwrap();

        graph.apply_patch_nodes(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("DISTANCE") {
            Some(Value::Number(n)) => assert_eq!(n, 0.0, "expected -5.0 to clamp to the min bound (0.0), not be silently dropped leaving DISTANCE at 3.0"),
            other => panic!("expected DISTANCE to still be a Number, got {:?}", other),
        }
    }

    #[test]
    fn add_mode_offsets_from_the_targets_value_at_mapping_time_not_from_zero() {
        // The exact reported scenario: RADIUS (here, DISTANCE) manually
        // set to a real value (64-ish) before mapping, then driven by an
        // animation whose own range is small (-5..5) - Replace mode would
        // make it snap down to near that tiny range; Add should offset
        // from the pre-existing 64, not replace it outright.
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(64.0), key_color: Cell::new(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 5.0, y: 0.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "DISTANCE", 0, PatchMode::Add).unwrap();

        graph.apply_patch_nodes(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("DISTANCE") {
            Some(Value::Number(n)) => assert_eq!(n, 69.0, "expected the captured base (64) plus the animation value (5), not just the animation value"),
            other => panic!("expected DISTANCE to still be a Number, got {:?}", other),
        }
    }

    #[test]
    fn subtract_mode_offsets_the_other_direction() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(64.0), key_color: Cell::new(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 5.0, y: 0.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "DISTANCE", 0, PatchMode::Subtract).unwrap();

        graph.apply_patch_nodes(&Context::default());

        let target_op = graph.get_node(&target).unwrap();
        match target_op.get_parameter("DISTANCE") {
            Some(Value::Number(n)) => assert_eq!(n, 59.0, "expected the captured base (64) minus the animation value (5)"),
            other => panic!("expected DISTANCE to still be a Number, got {:?}", other),
        }
    }

    #[test]
    fn switching_mode_on_an_already_mapped_property_keeps_the_original_captured_base() {
        // DISTANCE starts at 64, gets mapped in Replace mode (driving it
        // down to whatever the animation outputs), then the mapping is
        // switched to Add without ever clearing it - Add must still
        // offset from the *original* 64, not from Replace's already-
        // overwritten live value.
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(64.0), key_color: Cell::new(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 5.0, y: 0.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "DISTANCE", 0, PatchMode::Replace).unwrap();
        graph.apply_patch_nodes(&Context::default());
        // Live value is now 5.0 (Replace), not 64 - confirms the setup.
        match graph.get_node(&target).unwrap().get_parameter("DISTANCE") {
            Some(Value::Number(n)) => assert_eq!(n, 5.0, "setup check: Replace should have driven DISTANCE down to the raw animation value"),
            other => panic!("expected DISTANCE to still be a Number, got {:?}", other),
        }

        graph.set_patch_mapping(patch, "DISTANCE", 0, PatchMode::Add).unwrap();
        graph.apply_patch_nodes(&Context::default());

        match graph.get_node(&target).unwrap().get_parameter("DISTANCE") {
            Some(Value::Number(n)) => assert_eq!(n, 69.0, "expected Add to still offset from the original captured base (64), not from Replace's already-overwritten 5.0"),
            other => panic!("expected DISTANCE to still be a Number, got {:?}", other),
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
        graph.set_patch_mapping(patch, "KEY_COLOR.R", 0, PatchMode::Replace).unwrap();

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
        graph.set_patch_mapping(patch, "R", 0, PatchMode::Replace).unwrap();
        graph.set_patch_mapping(patch, "A", 1, PatchMode::Replace).unwrap();

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
    fn clearing_a_mapping_restores_the_targets_pre_mapping_value() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(64.0), key_color: Cell::new(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }) }));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 5.0, y: 0.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "DISTANCE", 0, PatchMode::Replace).unwrap();
        graph.apply_patch_nodes(&Context::default());
        // Confirm it's actually been driven away from 64 first.
        match graph.get_node(&target).unwrap().get_parameter("DISTANCE") {
            Some(Value::Number(n)) => assert_eq!(n, 5.0),
            other => panic!("expected DISTANCE to be replaced by 5.0, got {:?}", other),
        }

        graph.clear_patch_mapping(patch, "DISTANCE").unwrap();

        match graph.get_node(&target).unwrap().get_parameter("DISTANCE") {
            Some(Value::Number(n)) => assert_eq!(n, 64.0, "expected DISTANCE restored to its pre-mapping value (64), not stuck at the last animated value (5.0)"),
            other => panic!("expected DISTANCE to still be a Number, got {:?}", other),
        }
    }

    #[test]
    fn clearing_a_colour_channel_mapping_restores_just_that_channel() {
        let mut graph = Graph::new(4, 4);
        let target = graph.add_node(Box::new(FakeParameterizedTarget { distance: Cell::new(0.0), key_color: Cell::new(Color { r: 0.1, g: 0.2, b: 0.3, a: 0.4 }) }));
        let animation = graph.add_node(Box::new(FakeAnimationSource { x: 0.9, y: 0.0 }));
        let patch = graph.add_node(patch_node());
        graph.connect(patch, Input::Source, target).unwrap();
        graph.connect(patch, Input::Reference, animation).unwrap();
        graph.set_patch_mapping(patch, "KEY_COLOR.R", 0, PatchMode::Replace).unwrap();
        graph.apply_patch_nodes(&Context::default());

        graph.clear_patch_mapping(patch, "KEY_COLOR.R").unwrap();

        match graph.get_node(&target).unwrap().get_parameter("KEY_COLOR") {
            Some(Value::Color(c)) => assert!((c.r - 0.1).abs() < 1e-6, "expected R restored to its pre-mapping value (0.1), got {}", c.r),
            other => panic!("expected a Color, got {:?}", other),
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
        graph.set_patch_mapping(patch, "DISTANCE", 0, PatchMode::Replace).unwrap();

        graph.connect(patch, Input::Source, other_target).unwrap();

        let node = graph.resolve(patch).unwrap();
        assert!(node.animation_mappings.is_empty(), "rewiring SOURCE must drop mappings tied to the old target");
    }
}
