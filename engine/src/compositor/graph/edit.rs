// src/graph/edit.rs
//

use crate::compositor::{
    error::OperationError,
    input::Input,
    metadata::ParameterKind,
    operations::Operation,
};

use super::{
    Graph,
    node::NodeId,
    validate::{ValidationState, NodeValidation},
};

impl Graph {
    pub fn add_node(
        &mut self,
        operation: Box<dyn Operation>,
    ) -> NodeId {

        let node = super::node::Node {
            operation,
            inputs: Vec::new(),
            animation_mappings: Vec::new(),
        };

        let id = if let Some(index) = self.free.pop() {

            let generation =
                self.generations[index as usize];

            self.nodes[index as usize] = Some(node);
            
            // Ensure node_validation vector is large enough
            if index as usize >= self.node_validation.len() {
                self.node_validation.resize(index as usize + 1, NodeValidation::Valid);
            } else {
                self.node_validation[index as usize] = NodeValidation::Valid;
            }

            NodeId {
                index,
                generation,
            }

        } else {

            let index = self.nodes.len() as u32;

            self.nodes.push(Some(node));
            self.generations.push(0);
            self.node_validation.push(NodeValidation::Valid);

            NodeId {
                index,
                generation: 0,
            }
        };

        self.validation = ValidationState::Dirty;

        id
    }
    
    /// Get an immutable reference to a node's operation
    pub fn get_node(
        &self,
        id: &NodeId,
    ) -> Option<&dyn Operation> {
        let index = id.index as usize;
        
        if self.generations.get(index)? != &id.generation {
            return None;
        }
        
        self.nodes.get(index)?.as_ref().map(|node| node.operation.as_ref() as &dyn Operation)
    }

    /// Get a mutable reference to a node's operation
    pub fn get_node_mut(
        &mut self,
        id: &NodeId,
    ) -> Option<&mut dyn Operation> {
        let index = id.index as usize;
        
        if self.generations.get(index)? != &id.generation {
            return None;
        }
        
        self.nodes.get_mut(index)?.as_mut().map(|node| node.operation.as_mut() as &mut dyn Operation)
    }

    pub fn remove_node(
        &mut self,
        id: NodeId,
    ) -> Result<(), OperationError> {

        let index = id.index as usize;

        if self.resolve(id).is_none() {
            return Err(OperationError::UnknownNode);
        }

        self.nodes[index] = None;
        self.generations[index] =
            self.generations[index].wrapping_add(1);

        self.free.push(id.index);


        for node in self.nodes.iter_mut().flatten() {
            let had_wire_to_removed = node.inputs.iter().any(|(_, other)| *other == id);
            node.inputs.retain(|(_, other)| *other != id);

            // A PATCH node whose SOURCE or REFERENCE just got yanked out
            // from under it (the wired node was removed, not explicitly
            // rewired) needs the same mapping cleanup connect/disconnect
            // already do - the mappings named properties/outputs that
            // belonged to a node that no longer exists.
            if had_wire_to_removed {
                node.animation_mappings.clear();
            }
        }

        // Ensure node_validation vector is large enough
        if index >= self.node_validation.len() {
            self.node_validation.resize(index + 1, NodeValidation::Valid);
        }
        // Note: We don't need to update node_validation for the removed node
        // since it won't be accessible via the old NodeId due to generation check


        self.validation = ValidationState::Dirty;

        Ok(())
    }

    pub fn connect(
        &mut self,
        node: NodeId,
        input: Input,
        source: NodeId,
    ) -> Result<(), OperationError> {

        if self.resolve(node).is_none()
            || self.resolve(source).is_none()
        {
            return Err(OperationError::UnknownNode);
        }

        let target =
            self.resolve_mut(node)
            .ok_or(OperationError::UnknownNode)?;

        target.inputs.retain(|(key, _)| *key != input);

        target.inputs.push((input, source));

        // A PATCH node's own animation_mappings are property names that
        // belonged to whatever was previously wired to SOURCE (or output
        // indices scoped to whatever was wired to REFERENCE) - rewiring
        // either invalidates them, same reasoning
        // connect_animation_target used to clear mappings on a target
        // change. No-op for every non-PATCH node, since their
        // animation_mappings is always empty already.
        if input == Input::Source || input == Input::Reference {
            target.animation_mappings.clear();
        }

        self.validation = ValidationState::Dirty;

        Ok(())
    }

    pub fn disconnect(
        &mut self,
        node: NodeId,
        input: Input,
    ) -> Result<(), OperationError> {

        let target =
            self.resolve_mut(node)
            .ok_or(OperationError::UnknownNode)?;

        target.inputs.retain(|(key, _)| *key != input);

        if input == Input::Source || input == Input::Reference {
            target.animation_mappings.clear();
        }

        self.validation = ValidationState::Dirty;

        Ok(())
    }

    /// Which properties a PATCH node's currently-wired SOURCE (target)
    /// can have driven: every real Number parameter, by name; every real
    /// Color parameter, decomposed into "<NAME>.R"/"<NAME>.G"/"<NAME>.B"/
    /// "<NAME>.A"; or - if the target has neither (a plain pixel source
    /// with no parameters at all) - the four raw pixel channels "R"/"G"/
    /// "B"/"A" as a fallback, since there's nothing else to offer. Empty
    /// if no SOURCE is wired yet - never a fixed list unrelated to what's
    /// actually wired.
    pub fn available_patch_properties(&self, patch: NodeId) -> Vec<String> {
        let Some(node) = self.resolve(patch) else { return Vec::new() };
        let Some((_, target_id)) = node.inputs.iter().find(|(key, _)| *key == Input::Source) else {
            return Vec::new();
        };
        let Some(target_node) = self.resolve(*target_id) else { return Vec::new() };

        let mut properties = Vec::new();
        for parameter in target_node.operation.parameters() {
            match parameter.kind {
                ParameterKind::Number { .. } => properties.push(parameter.name.to_string()),
                ParameterKind::Color => {
                    for channel in ["R", "G", "B", "A"] {
                        properties.push(format!("{}.{}", parameter.name, channel));
                    }
                }
                _ => {}
            }
        }

        if properties.is_empty() {
            properties = ["R", "G", "B", "A"].iter().map(|s| s.to_string()).collect();
        }

        properties
    }

    /// Map one of a PATCH node's wired REFERENCE (animation source)'s
    /// outputs to one of its wired SOURCE (target)'s properties (or one
    /// of the four raw pixel channels, if the target has no real
    /// parameters - see `available_patch_properties`). Validates
    /// `property` against the live property list and `output_index`
    /// against the animation source's own declared output count.
    pub fn set_patch_mapping(&mut self, patch: NodeId, property: &str, output_index: usize) -> Result<(), OperationError> {
        let available = self.available_patch_properties(patch);
        if !available.iter().any(|p| p == property) {
            return Err(OperationError::UnknownParameter(property.to_string()));
        }

        let node = self.resolve(patch).ok_or(OperationError::UnknownNode)?;
        let Some((_, animation_id)) = node.inputs.iter().find(|(key, _)| *key == Input::Reference) else {
            return Err(OperationError::InvalidInputType(
                "Wire an animation source (REFERENCE) before mapping a property".into()
            ));
        };

        let animation_node = self.resolve(*animation_id).ok_or(OperationError::UnknownNode)?;
        let output_count = animation_node.operation.metadata().outputs.len();
        if output_index >= output_count {
            return Err(OperationError::InvalidInputType(format!(
                "Output index {} is out of range (this operation has {} outputs)",
                output_index, output_count
            )));
        }

        let node = self.resolve_mut(patch).ok_or(OperationError::UnknownNode)?;
        node.animation_mappings.retain(|(name, _)| name != property);
        node.animation_mappings.push((property.to_string(), output_index));

        self.validation = ValidationState::Dirty;

        Ok(())
    }

    /// Remove one property's mapping, leaving the rest of a PATCH node's
    /// mappings and its SOURCE/REFERENCE wiring untouched.
    pub fn clear_patch_mapping(&mut self, patch: NodeId, property: &str) -> Result<(), OperationError> {
        let node = self.resolve_mut(patch).ok_or(OperationError::UnknownNode)?;
        node.animation_mappings.retain(|(name, _)| name != property);

        self.validation = ValidationState::Dirty;

        Ok(())
    }

    pub fn validate(
        &mut self,
    ) -> Result<(), OperationError> {
        super::validate::validate_graph(self)
    }

    /// Get the validation state of a specific node
    /// Returns None if the node doesn't exist or the NodeId is stale
    pub fn node_validation(
        &self,
        id: NodeId,
    ) -> Option<NodeValidation> {
        super::validate::get_node_validation(self, id)
    }

    pub fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn set_resolution(
        &mut self,
        width: u32,
        height: u32,
    ) {
        self.width = width;
        self.height = height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::{
        Context,
        Value,
        metadata::{OperationCategory, OperationMetadata},
        OperationDescriptor,
    };
    use std::any::Any;

    struct Stub;

    impl Operation for Stub {
        fn descriptor(&self) -> OperationDescriptor {
            OperationDescriptor {
                id: "stub",
                menu: "TEST",
                label: "STUB",
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
                display_name: "Stub",
                category: OperationCategory::Source,
                inputs: vec![],
                outputs: vec![],
            }
        }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![])
        }
    }

    // Reproduces the exact reported bug: add a node, remove it, add another
    // node that reuses the freed slot - the new node must still resolve via
    // Graph::current_id (what App uses to turn a bare JS index into a real
    // NodeId), even though the slot's generation moved on.
    #[test]
    fn a_node_reusing_a_freed_slot_resolves_by_its_bare_index() {
        let mut graph = Graph::new(320, 240);

        let image_1 = graph.add_node(Box::new(Stub));
        let _blur_1 = graph.add_node(Box::new(Stub));

        graph.remove_node(image_1).unwrap();

        // MULTIPLY 1 reuses IMAGE 1's freed slot index, but with a bumped
        // generation.
        let multiply_1 = graph.add_node(Box::new(Stub));
        assert_eq!(multiply_1.index(), image_1.index());
        assert_ne!(multiply_1.generation, image_1.generation);

        let resolved = graph.current_id(multiply_1.index())
            .expect("current_id must resolve the live node at this slot");
        assert_eq!(resolved, multiply_1);
        assert!(graph.get_node(&resolved).is_some());

        // The stale NodeId from before removal must not resolve to the new
        // node occupying its old slot.
        assert!(graph.get_node(&image_1).is_none());
    }

    #[test]
    fn current_id_is_none_for_an_empty_slot() {
        let graph = Graph::new(320, 240);
        assert!(graph.current_id(0).is_none());
    }
}