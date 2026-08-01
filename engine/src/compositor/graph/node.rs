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

/// How a PATCH mapping combines the animation source's value with the
/// target property: overwrite it outright, or offset from the value the
/// property had before PATCH took it over (see `PatchMapping::base`).
/// There's no implicit default here - `set_patch_mapping` always takes
/// an explicit mode, chosen by the caller (the UI decides what to
/// propose for a brand-new mapping, currently Add - see
/// nodeEditContexts.js's own comment on why).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchMode {
    Replace,
    Add,
    Subtract,
}

/// One PATCH mapping: which of its wired SOURCE (target)'s properties
/// is driven, by which of its wired REFERENCE (animation source)'s
/// outputs, and how.
#[derive(Clone, Debug, PartialEq)]
pub struct PatchMapping {
    /// Owned `String`, not `&'static str`, since a Color parameter's
    /// decomposed channel name ("KEY_COLOR.R") is built at runtime, not
    /// one of the parameter's own fixed descriptor names.
    pub property: String,
    pub output_index: usize,
    pub mode: PatchMode,
    /// The target property's own value at the moment this mapping was
    /// first created (before PATCH ever wrote to it) - what Add/Subtract
    /// offset from. Captured once, not re-read every tick: re-reading the
    /// *current* value would read back whatever PATCH itself wrote last
    /// tick, so the property would drift by the animation's own value
    /// every tick instead of oscillating around a fixed centre. Switching
    /// modes (Replace -> Add -> Subtract) on an already-mapped property
    /// reuses this same captured base rather than recapturing - only a
    /// fresh mapping (the property was NONE beforehand) captures a new
    /// one. Unused (0.0) for the raw R/G/B/A pixel-substitution fallback,
    /// where there is no single scalar "current value" to offset from.
    pub base: f64,
}

pub struct Node {
    pub operation: Box<dyn Operation>,
    pub inputs: Vec<(Input, NodeId)>,
    /// PATCH-only: mappings authored on the PATCH node's own edit screen.
    /// Empty and unused for every non-PATCH operation.
    pub animation_mappings: Vec<PatchMapping>,
}