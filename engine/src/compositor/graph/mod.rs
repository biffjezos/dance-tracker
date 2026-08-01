// graph/mod.rs

mod describe;
mod drive;
mod edit;
pub mod node;
mod resolve;
mod validate;

pub use node::{Node, NodeId};
pub use validate::{NodeValidation, ValidationState};

pub struct Graph {
    pub(crate) nodes: Vec<Option<Node>>,
    pub(crate) generations: Vec<u32>,
    pub(crate) free: Vec<u32>,

    pub width: u32,
    pub height: u32,

    pub(crate) validation: validate::ValidationState,
    /// Per-node validation states, indexed by node index
    pub(crate) node_validation: Vec<validate::NodeValidation>,
}

impl Graph {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            nodes: Vec::new(),
            generations: Vec::new(),
            free: Vec::new(),
            width,
            height,
            validation: validate::ValidationState::Dirty,
            node_validation: Vec::new(),
        }
    }
}