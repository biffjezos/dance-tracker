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
    pub fn index(&self) -> u32 {
        self.index
    }
}

pub struct Node {
    pub operation: Box<dyn Operation>,
    pub inputs: Vec<(Input, NodeId)>,
}