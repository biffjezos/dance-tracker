// src/ccompositor/context.rs

use crate::compositor::bbox::Rect;
use crate::compositor::input::Input;
use crate::resources::manager::ResourceManager;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderQuality {
    Draft,
    Full,
}

impl Default for RenderQuality {
    fn default() -> Self {
        RenderQuality::Full
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Meta {
    pub frame: u64,
    pub fps: f32,
    pub time: f64,
    pub preview: bool,
    pub render_quality: RenderQuality,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Default)]
pub struct Context {
    pub meta: Meta,
    pub resources: ResourceManager,
    /// The bbox each wired input reported via its own `output_bbox()` -
    /// see BBOX_CONVENTIONS.md. Unlike `meta`/`resources`, this varies per
    /// node within a single tick: the executor overrides it on a per-node
    /// clone of the base `Context` immediately before calling `execute()`.
    /// Empty for any operation with no wired inputs, or when the executor
    /// hasn't been updated to populate it (e.g. directly-constructed test
    /// contexts) - `find_bbox` returning `None` is exactly equivalent to
    /// "no box was reported," so nothing breaks by omission.
    pub input_bboxes: Vec<(Input, Rect)>,
}