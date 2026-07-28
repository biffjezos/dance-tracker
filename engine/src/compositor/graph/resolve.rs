// graph/resolve.rs

use super::{
    Graph,
    node::{Node, NodeId},
};

impl Graph {

    pub(crate) fn resolve(
        &self,
        id: NodeId,
    ) -> Option<&Node> {

        let index = id.index as usize;

        if self.generations.get(index)? != &id.generation {
            return None;
        }

        self.nodes.get(index)?.as_ref()
    }


    pub(crate) fn resolve_mut(
        &mut self,
        id: NodeId,
    ) -> Option<&mut Node> {

        let index = id.index as usize;

        if self.generations.get(index)? != &id.generation {
            return None;
        }

        self.nodes.get_mut(index)?.as_mut()
    }
}