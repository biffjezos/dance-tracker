
use crate::compositor::graph::{ Graph, NodeId, PatchMode };
use wasm_bindgen::prelude::*;

pub fn js_err(err: OperationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}

// Milliseconds since the page's time origin - falls back to 0.0 rather than
// panicking if window/performance is ever unavailable, since losing the
// playback clock should never take down rendering.
pub fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

/*
Resolve a bare JS-supplied slot index into the current, generation-checked
NodeId for whatever node actually occupies that slot right now. JS only ever
holds a `u32` index (never a generation), so after a node has been removed
and its slot reused, reconstructing a NodeId with a stale/assumed generation
would silently fail to resolve the live node - this looks up the real
current generation via Graph::current_id instead.
*/
pub fn resolve_id(graph: &Graph, index: u32) -> Result<NodeId, JsValue> {
    graph.current_id(index)
        .ok_or_else(|| JsValue::from_str(&format!("Node {} not found", index)))
}

pub fn patch_mode_name(mode: PatchMode) -> &'static str {
    match mode {
        PatchMode::Replace => "REPLACE",
        PatchMode::Add => "ADD",
        PatchMode::Subtract => "SUBTRACT",
    }
}

pub fn parse_patch_mode(mode: &str) -> Result<PatchMode, JsValue> {
    match mode {
        "REPLACE" => Ok(PatchMode::Replace),
        "ADD" => Ok(PatchMode::Add),
        "SUBTRACT" => Ok(PatchMode::Subtract),
        other => Err(JsValue::from_str(&format!("Unknown PATCH mode: {}", other))),
    }
}
