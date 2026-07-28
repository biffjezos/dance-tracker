use crate::resources::manager::ResourceManager;

/*
Draft trades fidelity for speed (e.g. an operation could skip
antialiasing or subsample); Full is always correct. Nothing branches
on this yet - it exists so Meta's shape is already right for the first
operation that needs the distinction, same as Value::Mask/Image exist
ahead of anything producing them.
*/
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

/*
Per-tick, read-only facts about the evaluation that's currently
running - which frame/time this is, whether it's the preview or main
render pass, how much fidelity to spend, and the graph's current
render resolution (width/height mirror Graph::resolution() - see
App::context - so a source/generator operation reads its output size
from here every execute() call instead of baking one in at
construction time and needing the whole graph rebuilt to change it).
Distinct from ResourceManager below: Meta is cheap and rebuilt fresh
every tick, ResourceManager is the persistent, shared part of Context.
*/
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
