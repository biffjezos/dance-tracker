// graph/mod.rs

use crate::compositor::{
    error::OperationError,
    node::{Node, NodeId},
};

mod resolve;
mod edit;
mod describe;
mod validate;

pub mod node;

pub use node::{Node, NodeId};

pub struct Graph {
    pub(crate) nodes: Vec<Option<Node>>,
    pub(crate) generations: Vec<u32>,
    pub(crate) free: Vec<u32>,

    output: Option<NodeId>,

    pub width: u32,
    pub height: u32,

    validation: validate::ValidationState,
}

impl Graph {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            nodes: Vec::new(),
            generations: Vec::new(),
            free: Vec::new(),
            output: None,
            width,
            height,
            validation: validate::ValidationState::Dirty,
        }
    }
}