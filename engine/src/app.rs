// src/app.rs
#![cfg(target_arch = "wasm32")]

use serde::Serialize;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlVideoElement};

use crate::compositor::{
    Context,
    executors::{
        Execute,
        PreviewExecutor,
        RenderExecutor
    },
    graph::{ Graph, NodeId },
    Input,
    Meta,
    metadata::ParameterKind,
    OperationError,
    OperationRegistry,
    Value,
    value_to_text
};
use crate::graphics::{ Color, Image, ImageFormat };
use crate::dom::{ VideoElementPixelSource, write_frame_to_canvas};
use crate::operations::sources::ImageSource;

use crate::renderer::to_render_frame;
use crate::resources::manager::ResourceManager;
use std::sync::Arc;

fn js_err(err: OperationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}

/*
Resolve a bare JS-supplied slot index into the current, generation-checked
NodeId for whatever node actually occupies that slot right now. JS only ever
holds a `u32` index (never a generation), so after a node has been removed
and its slot reused, `NodeId::from_index` would reconstruct a stale
generation and silently fail to resolve the live node - this looks up the
real current generation instead.
*/
fn resolve_id(graph: &Graph, index: u32) -> Result<NodeId, JsValue> {
    graph.current_id(index)
        .ok_or_else(|| JsValue::from_str(&format!("Node {} not found", index)))
}

/*
What the UI is told about one editable parameter of a node. The options list
comes from the operation, so a selector can never offer a value the operation
does not accept.
*/
#[derive(Serialize)]
struct ParameterView {
    name: &'static str,
    kind: &'static str,
    options: &'static [&'static str],
    step: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    group: Option<&'static str>,
    value: String,
}

/*
What the UI is told about one input of a node: the wire name the operation
declares, and the node currently feeding it, if any.
*/
#[derive(Serialize)]
struct InputView {
    name: &'static str,
    source: Option<u32>,
}

#[wasm_bindgen]
pub struct App {
    graph: Graph,
    resources: ResourceManager,
    registry: OperationRegistry,
    frame_counter: u64,
    // Persists across ticks so its frame-to-frame cache actually helps -
    // a fresh RenderExecutor per tick would never see a "last tick" to
    // compare against.
    render_executor: RenderExecutor,
}


#[wasm_bindgen]
impl App {

    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> App {
        let mut registry = OperationRegistry::new();
        crate::operations::register::register_operations(&mut registry);
        App {
            graph: Graph::new(width, height),
            resources: ResourceManager::new(),
            registry,
            frame_counter: 0,
            render_executor: RenderExecutor::new(),
        }
    }
    pub fn get_operations(&self) -> JsValue {
        serde_wasm_bindgen::to_value(
            &self.registry.descriptors()
        )
        .unwrap()
    }

    pub fn execute_operation(&mut self, id: &str) {
        println!("Execute operation: {}", id);

        let Some(operation) = self.registry.create(id) else {
            println!("Operation not found: {}", id);
            return;
        };

        let ctx = self.context(true);

        let result = operation.execute(
            &ctx,
            &[]
        );

        match result {
            Ok(values) => {
                println!("Operation output: {:?}", values);
            }
            Err(err) => {
                println!("Operation failed: {:?}", err);
            }
        }
    }

    /// Create a node for any registered operation, by operation id.
    pub fn create_node( &mut self, operation_id: String, ) -> Result<u32, JsValue> {
        let operation = self
            .registry
            .create(&operation_id)
            .ok_or_else(|| {
                JsValue::from_str(
                    &format!("Unknown operation: {}", operation_id)
                )
            })?;

        let node_id = self.graph.add_node(operation);

        Ok(node_id.index())
    }

    /// Remove a node from the graph. Anyone wired to it is safely
    /// disconnected (Graph::remove_node strips the wire, it doesn't leave
    /// a dangling reference), and its render cache entry is dropped so it
    /// doesn't linger forever under a NodeId that can never be reused.
    pub fn remove_node(&mut self, node_id: u32) -> Result<(), JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;
        self.render_executor.invalidate(node_id);
        self.graph.remove_node(node_id).map_err(js_err)
    }

    /// Check if a node supports editing (has editable parameters)
    pub fn node_supports_edit(&self, node_id: u32) -> bool {
        let Some(node_id) = self.graph.current_id(node_id) else {
            return false;
        };
        self.graph.get_node(&node_id)
            .map(|op| op.supports_edit())
            .unwrap_or(false)
    }

    /// The editable parameters of a node, with their current values and the
    /// values they accept. The UI builds its controls from exactly this.
    pub fn node_parameters(&self, node_id: u32) -> Result<JsValue, JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;

        let operation = self.graph
            .get_node(&node_id)
            .ok_or_else(|| {
                JsValue::from_str(
                    &format!("Node {:?} not found", node_id)
                )
            })?;

        let views = operation
            .parameters()
            .into_iter()
            .map(|parameter| ParameterView {
                name: parameter.name,
                kind: parameter.kind.name(),
                options: parameter.kind.options(),
                step: parameter.kind.step(),
                min: parameter.kind.min(),
                max: parameter.kind.max(),
                group: parameter.group,
                value: operation
                    .get_parameter(parameter.name)
                    .map(|value| value_to_text(&value))
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>();

        serde_wasm_bindgen::to_value(&views)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// The inputs a node declares, and what is currently wired into each.
    pub fn node_inputs(&self, node_id: u32) -> Result<JsValue, JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;

        let description = self.graph
            .describe(node_id)
            .ok_or_else(|| {
                JsValue::from_str(
                    &format!("Node {:?} not found", node_id)
                )
            })?;

        let views = description
            .metadata
            .inputs
            .iter()
            .map(|key| InputView {
                name: key.name(),
                source: description
                    .inputs
                    .iter()
                    .find(|(wired, _)| wired == key)
                    .map(|(_, source)| source.index()),
            })
            .collect::<Vec<_>>();

        serde_wasm_bindgen::to_value(&views)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Wire one node's output into a named input of another node.
    pub fn connect_node_input(
        &mut self,
        node_id: u32,
        input: String,
        source_id: u32,
    ) -> Result<(), JsValue> {
        let key = Input::from_name(&input)
            .ok_or_else(|| {
                JsValue::from_str(
                    &format!("Unknown input: {}", input)
                )
            })?;

        let node_id = resolve_id(&self.graph, node_id)?;
        let source_id = resolve_id(&self.graph, source_id)?;

        self.graph
            .connect(
                node_id,
                key,
                source_id,
            )
            .map_err(js_err)
    }

    /// Remove whatever is wired into a named input of a node.
    pub fn disconnect_node_input(
        &mut self,
        node_id: u32,
        input: String,
    ) -> Result<(), JsValue> {
        let key = Input::from_name(&input)
            .ok_or_else(|| {
                JsValue::from_str(
                    &format!("Unknown input: {}", input)
                )
            })?;

        let node_id = resolve_id(&self.graph, node_id)?;

        self.graph
            .disconnect(node_id, key)
            .map_err(js_err)
    }

    /// Set image data on a specific ImageSource node
    /// Takes pixel data as Uint8Array, width, height
    pub fn set_image_on_node(
        &mut self,
        node_id: u32,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;

        // Get the operation from the graph
        let operation = self.graph.get_node_mut(&node_id)
            .ok_or_else(|| JsValue::from_str(&format!("Node {:?} not found", node_id)))?;

        // Downcast to ImageSource
        let image_source = operation.as_any_mut().downcast_mut::<ImageSource>()
            .ok_or_else(|| JsValue::from_str(&format!("Node {:?} is not an ImageSource", node_id)))?;

        // Create Image from the pixel data
        let image = Arc::new(Image {
            pixels: pixels.to_vec(),
            width,
            height,
            format: ImageFormat::Rgba8,
        });

        // Set the image on the source
        image_source.set_image(image);

        Ok(())
    }

    /*
    Hand a browser video element to any source node that reads pixels from one
    (a loaded video file, a live camera stream). The operation decides whether
    it accepts a pixel source; this boundary does not know or care which one
    it is talking to.
    */
    pub fn set_pixel_source_on_node(
        &mut self,
        node_id: u32,
        video: HtmlVideoElement,
        scratch_canvas: HtmlCanvasElement,
    ) -> Result<(), JsValue> {

        let node_id = resolve_id(&self.graph, node_id)?;

        let operation = self.graph
            .get_node_mut(&node_id)
            .ok_or_else(|| {
                JsValue::from_str(
                    &format!("Node {:?} not found", node_id)
                )
            })?;

        let pixel_source = VideoElementPixelSource {
            video,
            scratch_canvas,
        };

        operation
            .set_pixel_source(Arc::new(pixel_source))
            .map_err(js_err)
    }
    /// Update a parameter on a specific node. The incoming value is always a
    /// string (the UI never sends anything else); it's parsed here into the
    /// Value variant the parameter's own declared kind expects, then the
    /// operation owns validating it.
    pub fn update_node_parameter(
        &mut self,
        node_id: u32,
        parameter: String,
        value: String,
    ) -> Result<(), JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;

        let operation = self.graph.get_node_mut(&node_id)
            .ok_or_else(|| JsValue::from_str(&format!("Node {:?} not found", node_id)))?;

        let descriptor = operation
            .parameters()
            .into_iter()
            .find(|p| p.name == parameter)
            .ok_or_else(|| JsValue::from_str("Unknown parameter"))?;

        let value = match descriptor.kind {
            ParameterKind::Number { .. } => {
                Value::Number(
                    value.parse::<f64>()
                        .map_err(|_| JsValue::from_str("Invalid number"))?
                )
            }

            ParameterKind::Boolean => {
                Value::Boolean(
                    value.parse::<bool>()
                        .map_err(|_| JsValue::from_str("Invalid boolean"))?
                )
            }

            ParameterKind::Color => {
                Value::Color(
                    Color::from_hex(&value)
                        .ok_or_else(|| JsValue::from_str("Invalid color"))?
                )
            }

            ParameterKind::Text | ParameterKind::Enum(_) => {
                Value::Text(value)
            }
        };
        operation
            .set_parameter(&parameter, value)
            .map_err(js_err)
    }
    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.graph.set_resolution(width, height);
    }


    fn context(&self, preview: bool) -> Context {
        let (width, height) = self.graph.resolution();

        Context {
            meta: Meta {
                frame: self.frame_counter,
                preview,
                width,
                height,
                ..Meta::default()
            },
            resources: self.resources.clone(),
        }
    }


    pub fn render_tick(
        &mut self,
        output_node: usize,
        canvas: HtmlCanvasElement,
    ) -> Result<(), JsValue> {

        self.graph.validate().map_err(js_err)?;

        self.frame_counter += 1;

        let output_node = resolve_id(&self.graph, output_node as u32)?;

        let ctx = self.context(false);

        let values = self.render_executor
            .execute(
                &self.graph,
                output_node,
                &ctx,
            )
            .map_err(js_err)?;

        // Get the first value and convert to Frame at the renderer boundary
        let first_value = values.first()
            .ok_or_else(|| JsValue::from_str("No output value"))?;

        // Renderer boundary dispatch: convert any renderable Value to Frame
        let frame = to_render_frame(first_value)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to render frame: {:?}", e)))?;

        write_frame_to_canvas(&canvas, &frame)
    }


    pub fn preview_tick(
        &mut self,
        node: usize,
        canvas: HtmlCanvasElement,
    ) -> Result<(), JsValue> {

        self.graph.validate().map_err(js_err)?;

        let node = resolve_id(&self.graph, node as u32)?;

        let ctx = self.context(true);

        let executor = PreviewExecutor::default();

        let values = executor
            .execute(
                &self.graph,
                node,
                &ctx,
            )
            .map_err(js_err)?;

        // Get the first value and convert to Frame at the renderer boundary
        let first_value = values.first()
            .ok_or_else(|| JsValue::from_str("No output value"))?;

        // Renderer boundary dispatch: convert any renderable Value to Frame
        let frame = to_render_frame(first_value)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to render frame: {:?}", e)))?;

        write_frame_to_canvas(&canvas, &frame)
    }
}
