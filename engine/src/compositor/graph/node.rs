// src/compositor/graph/node.rs

use crate::compositor::{
    input::Input,
    operations::Operation,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl NodeId {
    pub fn from_index(index: u32) -> Self {
        Self {
            index,
            generation: 0,
        }
    }

    pub fn index(&self) -> u32 {
        self.index
    }
}

pub struct Node {
    pub operation: Box<dyn Operation>,
    pub inputs: Vec<(Input, NodeId)>,
}

impl Node {
    pub fn input(&self, key: Input) -> Option<NodeId> {
        self.inputs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, id)| *id)
    }
}