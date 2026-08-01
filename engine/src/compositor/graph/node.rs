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
    /// PATCH-only: (property_name, output_index) pairs authored on the
    /// PATCH node's own edit screen, mapping one of its wired REFERENCE
    /// (animation source)'s outputs to one of its wired SOURCE (target)'s
    /// properties. Owned `String`, not `&'static str`, since a Color
    /// parameter's decomposed channel name ("KEY_COLOR.R") is built at
    /// runtime, not one of the parameter's own fixed descriptor names.
    /// Empty and unused for every non-PATCH operation.
    pub animation_mappings: Vec<(String, usize)>,
}