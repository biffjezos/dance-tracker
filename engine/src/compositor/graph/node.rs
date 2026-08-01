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
    /// Which node this one drives, if it's an animation-category
    /// operation with a target selected - the driver owns this
    /// reference, not the target (see ANIMATION_IMPLEMENTATION_PLAN.md's
    /// Phase C: the UI authors this from the driver's own edit screen,
    /// same as picking any other node reference). `None` for every
    /// non-driving node, and for a driver with no target picked yet.
    pub animation_target: Option<NodeId>,
    /// (output_index, target_parameter_name) pairs - which of this
    /// driver's own outputs controls which Number parameter on
    /// `animation_target`. Sparse: an output with no mapping simply has
    /// no entry. Cleared whenever `animation_target` changes, since the
    /// parameter names belonged to the old target.
    pub animation_mappings: Vec<(usize, &'static str)>,
}