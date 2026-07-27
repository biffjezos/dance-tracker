/*
Caches per-source scratch resources so rebuilding the graph (app.js
does this on every settings change anywhere in the app, not just
video-related ones - see app.js's rebuildGraph doc comment) doesn't
recreate a fresh decode target for a video element it has already
seen. A cheap Rc<RefCell<..>> handle, not an owned cache - App keeps
one persistent ResourceManager and hands out clones (shared, not
copies) into every Context it builds, so the cache actually survives
across calls instead of being thrown away with the Context that held
it.

No Any/downcasting here either: the cache holds one concrete resource
kind (a scratch canvas paired with the video element it belongs to),
not a generic type-erased slot. A linear scan over a handful of
entries (there are only ever a few distinct HtmlVideoElements in a
session - the camera feed, maybe one or two loaded files) beats
needing Hash/Eq on a JsValue-wrapping type for no real benefit.
*/
use std::cell::RefCell;
use std::rc::Rc;
#[derive(Clone)]
pub struct ResourceManager {
    // Only read by scratch_canvas_for below, which is wasm32-only -
    // the struct itself stays available on every target since Context
    // (used natively too, by every operation's tests) holds one
    // unconditionally.
    #[allow(dead_code)]
    inner: Rc<RefCell<Inner>>,
}
struct Inner {
    #[cfg(target_arch = "wasm32")]
    scratch_canvases: Vec<(web_sys::HtmlVideoElement, web_sys::HtmlCanvasElement)>,
}
impl ResourceManager {
    pub fn new() -> Self {
        ResourceManager {
            inner: Rc::new(
                RefCell::new(Inner {
                    #[cfg(target_arch = "wasm32")]
                    scratch_canvases: Vec::new(),
                })
            ),
        }
    }
}
impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(target_arch = "wasm32")]
impl ResourceManager {
    /*
    Returns the scratch canvas already associated with this exact video
    element, if any, creating and caching one otherwise.
    */
    pub fn scratch_canvas_for(
        &self,
        video: &web_sys::HtmlVideoElement
    ) -> Result<web_sys::HtmlCanvasElement, wasm_bindgen::JsValue> {
        let mut inner = self.inner.borrow_mut();
        if let Some((_, canvas)) = inner.scratch_canvases.iter().find(|(v, _)| v == video) {
            return Ok(canvas.clone());
        }
        let document = web_sys
            ::window()
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("no window"))?
            .document()
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("no document"))?;
        let canvas: web_sys::HtmlCanvasElement = wasm_bindgen::JsCast::dyn_into(
            document.create_element("canvas")?
        )?;
        inner.scratch_canvases.push((video.clone(), canvas.clone()));
        Ok(canvas)
    }
}
