use crate::compositor::{
    input::Input,
    operations::Operation,
};

pub type NodeId = usize;

pub struct Node {
    pub operation: Box<dyn Operation>,
    pub inputs: Vec<(Input, NodeId)>,
}


