// src/graph/resolve.rs

use super::{
    Graph,
    node::{Node, NodeId},
};

impl Graph {

    /// Resolve a bare slot index (as passed across the WASM boundary, which
    /// only ever carries a `u32` index, never a generation) into the
    /// current, generation-checked `NodeId` for whatever node currently
    /// occupies that slot. Returns `None` if the slot is empty - this is
    /// distinct from `NodeId::from_index`, which always assumes generation
    /// 0 and silently mismatches a live node after its slot's generation
    /// has been bumped by a prior removal.
    pub fn current_id(
        &self,
        index: u32,
    ) -> Option<NodeId> {
        let generation = *self.generations.get(index as usize)?;
        self.nodes.get(index as usize)?.as_ref()?;

        Some(NodeId {
            index,
            generation,
        })
    }

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