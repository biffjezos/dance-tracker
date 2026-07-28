// graph/describe.rs

use crate::compositor::{
    metadata::OperationMetadata,
    value::Value,
    input::Input,
};

use super::{
    Graph,
    node::{Node, NodeId},
};


pub struct NodeDescription {
    pub id: NodeId,
    pub metadata: OperationMetadata,
    pub parameters: Vec<(&'static str, Value)>,
    pub inputs: Vec<(Input, NodeId)>,
}

impl NodeDescription {

    pub fn from_node(
        id: NodeId,
        node: &Node,
    ) -> Self {

        let parameters =
            node.operation
                .parameters()
                .into_iter()
                .filter_map(|p| {
                    node.operation
                        .get_parameter(p.name)
                        .map(|v| (p.name, v))
                })
                .collect();


        Self {
            id,
            metadata: node.operation.metadata(),
            parameters,
            inputs: node.inputs.clone(),
        }
    }
}


impl Graph {

    pub fn describe(
        &self,
        id: NodeId,
    ) -> Option<NodeDescription> {

        let node = self.resolve(id)?;

        Some(NodeDescription::from_node(id, node))
    }
}