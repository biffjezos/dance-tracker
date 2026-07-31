// src/graph/edit.rs
//

use crate::compositor::{
    error::OperationError,
    input::Input,
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
            node.inputs.retain(|(_, other)| *other != id);
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