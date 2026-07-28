// src/ccompositor/context.rs

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
}
