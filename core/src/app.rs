/*
The JS-callable surface. wasm-bindgen can't export Box<dyn Operation>
or the Graph/Node types directly (no trait objects, no generic types
across the boundary), so this is a single opaque App struct JS holds a
handle to, with plain methods (numbers/strings/DOM element handles in,
a NodeId - just usize - out) that build the graph and drive the two
real-time executors. This is the "operations/executor -> UI" direction
reversed: JS calls these methods, these methods call the executors,
the executors call the operations - matching the architecture, just
written from the boundary inward instead of the graph outward.
*/
#![cfg(target_arch = "wasm32")]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlVideoElement};

use crate::compositor::{Context, Input, Meta, OperationError};
use crate::dom::{write_frame_to_canvas, VideoElementPixelSource};
use crate::graph::{Graph, Node, NodeId};
use crate::operations::composite::apply_mask::Channel as MaskChannel;
use crate::operations::composite::{ApplyMask, BlendMode, Compose};
use crate::operations::executor::{Execute, PreviewExecutor, RenderExecutor, SimpleExecutor};
use crate::operations::generators::{Ghost, Rings, Text};
use crate::operations::masks::{Chroma, Difference, Fill};
use crate::operations::sources::video::VideoSource;
use crate::operations::sources::CapturedFrame;
use crate::operations::{expect_frame, expect_frame_arc, Frame};
use crate::resource_manager::ResourceManager;

fn js_err(err: OperationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}

fn parse_channel(channel: &str) -> MaskChannel {
    match channel {
        "red" => MaskChannel::Red,
        "green" => MaskChannel::Green,
        "blue" => MaskChannel::Blue,
        _ => MaskChannel::Alpha,
    }
}

fn parse_blend_mode(mode: &str) -> BlendMode {
    match mode {
        "screen" => BlendMode::Screen,
        "multiply" => BlendMode::Multiply,
        _ => BlendMode::Over,
    }
}

fn parse_fill(fill_video: bool, r: u8, g: u8, b: u8) -> Fill {
    if fill_video {
        Fill::Video
    } else {
        Fill::Solid(r, g, b)
    }
}

#[wasm_bindgen]
pub struct App {
    graph: Graph,
    captured: HashMap<NodeId, Rc<RefCell<Option<Arc<Frame>>>>>,
    difference_source: HashMap<NodeId, NodeId>,
    width: u32,
    height: u32,
    resources: ResourceManager,
    /*
    Only render_tick advances this - preview_tick and
    capture_background read the current value rather than each
    keeping their own notion of "which frame is this".
    */
    frame_counter: Cell<u64>,
}

#[wasm_bindgen]
impl App {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> App {
        App {
            graph: Graph::new(width, height),
            captured: HashMap::new(),
            difference_source: HashMap::new(),
            width,
            height,
            resources: ResourceManager::new(),
            frame_counter: Cell::new(0),
        }
    }

    /*
    Changing resolution never requires rebuilding the graph - it just
    updates the one shared value every operation reads through Context
    on its next execute() call. Also updates width/height (still used
    directly by add_video_source's own letterboxing target) so a
    resize takes effect for future add_video_source calls even before
    every operation reads its size from Context instead of a
    constructor argument.
    */
    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.graph.set_resolution(width, height);
        self.width = width;
        self.height = height;
    }

    /*
    A fresh Context sharing the persistent ResourceManager (a clone is
    just another handle to the same cache, not a new one) plus a Meta
    stamped with the current frame count, which pass this is for, and
    the graph's current render resolution - fps/time stay at Meta's
    defaults, nothing reads them yet.
    */
    fn context(&self, preview: bool) -> Context {
        let (width, height) = self.graph.resolution();

        Context {
            meta: Meta {
                frame: self.frame_counter.get(),
                preview,
                width,
                height,
                ..Meta::default()
            },
            resources: self.resources.clone(),
        }
    }

    /*
    ==================================================
    SOURCES
    ==================================================
    */

    /*
    Letterboxed (containFit) to the project's own width/height, same
    as every other node's fixed frame size - a video's native
    resolution almost never matches it.
    */
    pub fn add_video_source(&mut self, video: HtmlVideoElement) -> Result<usize, JsValue> {
        // rebuildGraph() re-adds a source for the same HtmlVideoElement
        // on every settings change anywhere in the app, not just video
        // ones - reusing the scratch canvas here (instead of minting a
        // fresh one every time) is exactly the redundant-reload the
        // ResourceManager exists to avoid.
        let scratch = self.resources.scratch_canvas_for(&video)?;

        let pixels = VideoElementPixelSource {
            video,
            scratch_canvas: scratch,
            target_width: self.width,
            target_height: self.height,
        };

        let node = Node {
            operation: Box::new(VideoSource { pixels: Box::new(pixels) }),
            inputs: vec![],
        };

        Ok(self.graph.add_node(node))
    }

    /*
    ==================================================
    MASKS (KEY)
    ==================================================
    */

    pub fn add_chroma(
        &mut self,
        source: usize,
        key_r: u8,
        key_g: u8,
        key_b: u8,
        threshold: u32,
        fill_video: bool,
        fill_r: u8,
        fill_g: u8,
        fill_b: u8,
    ) -> usize {
        let node = Node {
            operation: Box::new(Chroma {
                key_colour: (key_r, key_g, key_b),
                threshold,
                fill: parse_fill(fill_video, fill_r, fill_g, fill_b),
            }),
            inputs: vec![(Input::Source, source)],
        };

        self.graph.add_node(node)
    }

    /*
    Also creates the CapturedFrame reference node under the hood and
    wires it as Input::Reference - see capture_background for how the
    user's CAPTURE BACKGROUND click feeds it a frame.
    */
    pub fn add_difference(
        &mut self,
        source: usize,
        threshold: u32,
        fill_video: bool,
        fill_r: u8,
        fill_g: u8,
        fill_b: u8,
    ) -> usize {
        let captured_op = CapturedFrame::new();
        let handle = captured_op.handle();

        let captured_node = Node {
            operation: Box::new(captured_op),
            inputs: vec![],
        };

        let captured_id = self.graph.add_node(captured_node);

        self.captured.insert(captured_id, handle);

        let node = Node {
            operation: Box::new(Difference {
                threshold,
                fill: parse_fill(fill_video, fill_r, fill_g, fill_b),
            }),
            inputs: vec![(Input::Source, source), (Input::Reference, captured_id)],
        };

        let difference_id = self.graph.add_node(node);

        self.difference_source.insert(difference_id, captured_id);

        difference_id
    }

    /*
    Runs the difference node's own video input through SimpleExecutor
    (a one-off pull, not a per-frame evaluation) and stores the result
    into its CapturedFrame reference node - "CAPTURE BACKGROUND".
    */
    pub fn capture_background(&mut self, difference_node: usize) -> Result<(), JsValue> {
        self.graph.validate().map_err(js_err)?;

        let source_id = self.graph.nodes[difference_node]
            .input(Input::Source)
            .ok_or_else(|| JsValue::from_str("difference node has no source input"))?;

        let ctx = self.context(false);
        let executor = SimpleExecutor;

        let values = executor
            .execute(&self.graph, source_id, &ctx)
            .map_err(js_err)?;

        let frame = expect_frame_arc(values.first()).map_err(js_err)?;

        let captured_id = *self
            .difference_source
            .get(&difference_node)
            .ok_or_else(|| JsValue::from_str("not a difference node"))?;

        let handle = self
            .captured
            .get(&captured_id)
            .ok_or_else(|| JsValue::from_str("no captured-frame handle"))?;

        *handle.borrow_mut() = Some(frame);

        Ok(())
    }

    pub fn add_apply_mask(&mut self, content: usize, mask: usize, channel: &str) -> usize {
        let node = Node {
            operation: Box::new(ApplyMask { channel: parse_channel(channel) }),
            inputs: vec![(Input::Content, content), (Input::Mask, mask)],
        };

        self.graph.add_node(node)
    }

    /*
    ==================================================
    COMPOSITE (BACKGROUND)
    ==================================================
    */

    pub fn add_compose(&mut self, foreground: usize, background: usize, mode: &str) -> usize {
        let node = Node {
            operation: Box::new(Compose { mode: parse_blend_mode(mode) }),
            inputs: vec![(Input::Foreground, foreground), (Input::Background, background)],
        };

        self.graph.add_node(node)
    }

    /*
    ==================================================
    GENERATORS
    ==================================================
    */

    pub fn add_rings(
        &mut self,
        width: u32,
        height: u32,
        count: u32,
        rings_per_group: u32,
        spacing: f64,
        size: f64,
        stroke_width: f64,
    ) -> Result<usize, JsValue> {
        let colours = vec![
            "rgb(255,0,255)".to_string(),
            "rgb(0,255,80)".to_string(),
        ];

        let rings = Rings::new(
            width,
            height,
            count,
            rings_per_group,
            spacing,
            size,
            stroke_width,
            colours,
            None,
        )?;

        let node = Node { operation: Box::new(rings), inputs: vec![] };

        Ok(self.graph.add_node(node))
    }

    pub fn add_ghost(&mut self, source: usize, count: usize, alpha: f32, delay_ticks: u32) -> usize {
        let node = Node {
            operation: Box::new(Ghost::new(count, alpha, delay_ticks)),
            inputs: vec![(Input::Source, source)],
        };

        self.graph.add_node(node)
    }

    pub fn add_text(
        &mut self,
        width: u32,
        height: u32,
        content: String,
        colour: String,
        size: f64,
    ) -> Result<usize, JsValue> {
        let text = Text::new(width, height, content, colour, size)?;

        let node = Node { operation: Box::new(text), inputs: vec![] };

        Ok(self.graph.add_node(node))
    }

    pub fn set_text_content(&mut self, node_id: usize, content: String) {
        if let Some(text) = self.graph.nodes[node_id]
            .operation
            .as_any_mut()
            .downcast_mut::<Text>()
        {
            text.content = content;
        }
    }

    /*
    ==================================================
    CONTROLS (TRANSPORT)

    Plain HtmlVideoElement calls, not graph Operations - these never
    touch the graph, a Value, or a Context, so wrapping them in the
    Operation trait bought nothing but as_any/as_any_mut boilerplate
    for something JS could've called on the video element directly.
    ==================================================
    */

    pub fn play(&self, video: HtmlVideoElement) -> Result<(), JsValue> {
        let _ = video.play()?;
        Ok(())
    }

    pub fn stop(&self, video: HtmlVideoElement) -> Result<(), JsValue> {
        video.pause()?;
        Ok(())
    }

    /*
    Covers MINUTE +/SECOND +/FRAME + alike - just different seconds
    values (60.0, 1.0, 1.0/30.0) passed in from JS.
    */
    pub fn forward(&self, video: HtmlVideoElement, seconds: f64) -> Result<(), JsValue> {
        video.set_current_time(video.current_time() + seconds);
        Ok(())
    }

    pub fn rewind(&self, video: HtmlVideoElement, seconds: f64) -> Result<(), JsValue> {
        video.set_current_time((video.current_time() - seconds).max(0.0));
        Ok(())
    }

    /*
    ==================================================
    PER-TICK EVALUATION
    ==================================================
    */

    pub fn render_tick(&self, output_node: usize, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        self.graph.validate().map_err(js_err)?;

        self.frame_counter.set(self.frame_counter.get() + 1);
        let ctx = self.context(false);
        let executor = RenderExecutor;

        let values = executor
            .execute(&self.graph, output_node, &ctx)
            .map_err(js_err)?;

        let frame = expect_frame(values.first()).map_err(js_err)?;

        write_frame_to_canvas(&canvas, frame)
    }

    pub fn preview_tick(&self, node: usize, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        self.graph.validate().map_err(js_err)?;

        let ctx = self.context(true);
        let executor = PreviewExecutor;

        let values = executor.execute(&self.graph, node, &ctx).map_err(js_err)?;

        let frame = expect_frame(values.first()).map_err(js_err)?;

        write_frame_to_canvas(&canvas, frame)
    }
}

