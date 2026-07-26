/*
Unchanged from the stub. A Node's inputs are positional (Vec<NodeId>),
not named fields - what position 0 vs 1 means is entirely up to the
concrete Operation reading them (e.g. Compose treats inputs[0] as the
foreground, inputs[1] as the background).
*/

use crate::compositor::Operation;

pub type NodeId = usize;

pub struct Node {
    pub operation: Box<dyn Operation>,
    pub inputs: Vec<NodeId>,
}

pub struct Graph {
    pub nodes: Vec<Node>,
}

impl Graph {
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }
}
