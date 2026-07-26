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

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlVideoElement};

use crate::compositor::{Context, Operation, OperationError};
use crate::dom::{write_frame_to_canvas, VideoElementPixelSource};
use crate::graph::{Graph, Node, NodeId};
use crate::operations::composite::apply_mask::Channel as MaskChannel;
use crate::operations::composite::{ApplyMask, BlendMode, Compose};
use crate::operations::controls::{Forward, Play, Rewind, Stop};
use crate::operations::executor::{Execute, PreviewExecutor, RenderExecutor, SimpleExecutor};
use crate::operations::generators::{Ghost, Rings, Text};
use crate::operations::masks::{Chroma, Difference, Fill};
use crate::operations::sources::video::VideoSource;
use crate::operations::sources::CapturedFrame;
use crate::operations::{downcast_frame, Frame};

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
    captured: HashMap<NodeId, Rc<RefCell<Option<Frame>>>>,
    difference_source: HashMap<NodeId, NodeId>,
    width: u32,
    height: u32,
}

#[wasm_bindgen]
impl App {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> App {
        App {
            graph: Graph { nodes: vec![] },
            captured: HashMap::new(),
            difference_source: HashMap::new(),
            width,
            height,
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
        let document = web_sys::window()
            .ok_or_else(|| JsValue::from_str("no window"))?
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;

        let scratch = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;

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
            inputs: vec![source],
        };

        self.graph.add_node(node)
    }

    /*
    Also creates the CapturedFrame reference node under the hood and
    wires it as input[1] - see capture_background for how the user's
    CAPTURE BACKGROUND click feeds it a frame.
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
            inputs: vec![source, captured_id],
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
        let source_id = self.graph.nodes[difference_node].inputs[0];

        let ctx = Context { data: Box::new(()) };
        let executor = SimpleExecutor;

        let values = executor
            .execute(&self.graph, source_id, &ctx)
            .map_err(js_err)?;

        let frame = downcast_frame(values.first()).map_err(js_err)?.clone();

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
            inputs: vec![content, mask],
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
            inputs: vec![foreground, background],
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
            inputs: vec![source],
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
    ==================================================
    */

    pub fn play(&self, video: HtmlVideoElement) -> Result<(), JsValue> {
        let ctx = Context { data: Box::new(()) };
        Play { video }.execute(&ctx, &[]).map_err(js_err)?;
        Ok(())
    }

    pub fn stop(&self, video: HtmlVideoElement) -> Result<(), JsValue> {
        let ctx = Context { data: Box::new(()) };
        Stop { video }.execute(&ctx, &[]).map_err(js_err)?;
        Ok(())
    }

    pub fn forward(&self, video: HtmlVideoElement, seconds: f64) -> Result<(), JsValue> {
        let ctx = Context { data: Box::new(()) };
        Forward { video, seconds }.execute(&ctx, &[]).map_err(js_err)?;
        Ok(())
    }

    pub fn rewind(&self, video: HtmlVideoElement, seconds: f64) -> Result<(), JsValue> {
        let ctx = Context { data: Box::new(()) };
        Rewind { video, seconds }.execute(&ctx, &[]).map_err(js_err)?;
        Ok(())
    }

    /*
    ==================================================
    PER-TICK EVALUATION
    ==================================================
    */

    pub fn render_tick(&self, output_node: usize, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        let ctx = Context { data: Box::new(()) };
        let executor = RenderExecutor;

        let values = executor
            .execute(&self.graph, output_node, &ctx)
            .map_err(js_err)?;

        let frame = downcast_frame(values.first()).map_err(js_err)?;

        write_frame_to_canvas(&canvas, frame)
    }

    pub fn preview_tick(&self, node: usize, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        let ctx = Context { data: Box::new(()) };
        let executor = PreviewExecutor;

        let values = executor.execute(&self.graph, node, &ctx).map_err(js_err)?;

        let frame = downcast_frame(values.first()).map_err(js_err)?;

        write_frame_to_canvas(&canvas, frame)
    }
}

