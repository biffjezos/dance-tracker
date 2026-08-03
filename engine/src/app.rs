// src/app.rs
#![cfg(target_arch = "wasm32")]

use serde::Serialize;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlVideoElement};

use crate::compositor::{
    ComputeMode,
    Context,
    executors::{
        Execute,
        PreviewExecutor,
        RenderExecutor
    },
    graph::{ Graph, NodeId, NodeValidation, PatchMode },
    Input,
    Meta,
    OperationDescriptor,
    metadata::ParameterKind,
    OperationError,
    OperationRegistry,
    system::{ SystemMenuDescriptor },
    Value,
    value_to_text
};
use crate::compute::backend::ComputeBackend;
use crate::graphics::{ Color, U8Image, ImageFormat };
use crate::dom::{ VideoElementPixelSource, write_frame_to_canvas};
use crate::operations::sources::ImageSource;

use crate::renderer::to_render_frame;
use crate::resources::manager::ResourceManager;
use std::sync::Arc;

fn js_err(err: OperationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}

// Milliseconds since the page's time origin - falls back to 0.0 rather than
// panicking if window/performance is ever unavailable, since losing the
// playback clock should never take down rendering.
fn now_ms() -> f64 {
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
fn resolve_id(graph: &Graph, index: u32) -> Result<NodeId, JsValue> {
    graph.current_id(index)
        .ok_or_else(|| JsValue::from_str(&format!("Node {} not found", index)))
}

/*
What the UI is told about one registered operation: its own descriptor
fields plus the category its metadata() declares - carried as a plain
string (not the enum) since this is the JS boundary, and added ahead of
any UI code reading it yet so a future generic menu/list grouping (Phase 4)
has it available without another engine change.
*/
#[derive(Serialize)]
struct OperationView {
    id: &'static str,
    menu: &'static str,
    label: &'static str,
    action: Option<&'static str>,
    ui_action: Option<&'static str>,
    create_node: Option<&'static str>,
    category: &'static str,
    submenu: Option<&'static str>,
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
    // Which OutputKind tags (see OutputKind::as_str()) may be wired into
    // this input - empty means unrestricted (every real node is a valid
    // candidate).
    accepts: Vec<&'static str>,
}

/*
What the UI is told about one declared output of a node: its index (what
`set_patch_mapping` addresses it by), its human-readable label (see
`Operation::output_names`), and its OutputKind tag (see OutputKind::as_str()).
*/
#[derive(Serialize)]
struct OutputView {
    index: u32,
    name: String,
    kind: String,
}

/*
What the UI is told about one PATCH property's current mapping - which
REFERENCE output drives it, and how (REPLACE/ADD/SUBTRACT). `patch_mapping`
returns this (or nothing at all) per property.
*/
#[derive(Serialize)]
struct PatchMappingView {
    output_index: u32,
    mode: &'static str,
}

fn patch_mode_name(mode: PatchMode) -> &'static str {
    match mode {
        PatchMode::Replace => "REPLACE",
        PatchMode::Add => "ADD",
        PatchMode::Subtract => "SUBTRACT",
    }
}

fn parse_patch_mode(mode: &str) -> Result<PatchMode, JsValue> {
    match mode {
        "REPLACE" => Ok(PatchMode::Replace),
        "ADD" => Ok(PatchMode::Add),
        "SUBTRACT" => Ok(PatchMode::Subtract),
        other => Err(JsValue::from_str(&format!("Unknown PATCH mode: {}", other))),
    }
}

/*
Whether a node is safe to evaluate, translated from the engine's internal
NodeValidation (which carries NodeId, not JS-safe on its own) into a tag the
UI can match on plus a human-readable detail string - e.g. so the NODES
list can badge a node with a dangling or cyclic wire instead of the user
only finding out when the whole graph refuses to render.
*/
#[derive(Serialize)]
struct NodeValidationView {
    state: &'static str,
    detail: Option<String>,
}

impl From<NodeValidation> for NodeValidationView {
    fn from(state: NodeValidation) -> Self {
        match state {
            NodeValidation::Valid => Self { state: "valid", detail: None },
            NodeValidation::MissingInput(input) => Self {
                state: "missing_input",
                detail: Some(input.name().to_string()),
            },
            NodeValidation::UnknownInput(id) => Self {
                state: "unknown_input",
                detail: Some(id.index().to_string()),
            },
            NodeValidation::InvalidDependency(id) => Self {
                state: "invalid_dependency",
                detail: Some(id.index().to_string()),
            },
            NodeValidation::Cycle => Self { state: "cycle", detail: None },
        }
    }
}

#[wasm_bindgen]
pub struct App {
    graph: Graph,
    resources: ResourceManager,
    registry: OperationRegistry,
    compute_mode: ComputeMode,
    frame_counter: u64,
    // Persists across ticks so its frame-to-frame cache actually helps -
    // a fresh RenderExecutor per tick would never see a "last tick" to
    // compare against.
    render_executor: RenderExecutor,
    // performance.now() timestamp (ms) captured once at construction, so
    // context() can report meta.time as seconds-since-start rather than an
    // absolute wall-clock value - Video::frame_at expects 0.0 to mean "the
    // start of playback", not "the start of the Unix epoch".
    start_time_ms: f64,
    // Whether the last render_tick's output value was an out-of-gamut
    // FloatImage (see graphics::FloatImage) - the render boundary still
    // clamps it for display (a canvas can only ever show a bounded image),
    // but this is what lets the UI warn that display isn't the same thing
    // as an explicit CLAMP, so the out-of-range data isn't silently lost
    // on the user just because something is still visible.
    output_out_of_gamut: bool,
    compute: Arc<dyn ComputeBackend>,
    system_menus: Vec<SystemMenuDescriptor>,
}


#[wasm_bindgen]
impl App {

    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> App {
        let mut registry = OperationRegistry::new();
        let system_menus = crate::compositor::system::SystemMenu::descriptors();
        crate::operations::register::register_operations(&mut registry);

        let compute_mode = ComputeMode::Auto;

        let compute: Arc<dyn ComputeBackend> = crate::compute::create_backend(compute_mode);

        App {
            graph: Graph::new(width, height),
            resources: ResourceManager::new(),
            registry,
            frame_counter: 0,
            render_executor: RenderExecutor::new(),
            start_time_ms: now_ms(),
            output_out_of_gamut: false,
            compute,
            compute_mode,
            system_menus
        }
    }
    pub fn set_compute_mode(&mut self, mode: String) -> Result<(), JsValue> {
        let new_mode = match mode.as_str() {
            "CPU" => ComputeMode::Cpu,
            "GPU" => ComputeMode::Gpu,
            "AUTO" => ComputeMode::Auto,
            _ => return Err(JsValue::from_str("Unknown compute mode")),
        };

        self.compute = crate::compute::create_backend(new_mode);
        self.compute_mode = new_mode;

        Ok(())
    }

    pub fn get_system_menus(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&crate::compositor::system::SystemMenu::descriptors())
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }
    // Returns Result (not JsValue directly) so a serialization failure
    // becomes a catchable JS error instead of panicking the WASM instance,
    // matching every other JS-facing method in this file.
    pub fn get_operations(&self) -> Result<JsValue, JsValue> {
        let views: Vec<OperationView> = self.registry
            .describe_all()
            .into_iter()
            .map(|(descriptor, category)| OperationView {
                id: descriptor.id,
                menu: descriptor.menu,
                label: descriptor.label,
                action: descriptor.action,
                ui_action: descriptor.ui_action,
                create_node: descriptor.create_node,
                category: category.as_str(),
                submenu: descriptor.submenu,
            })
            .collect();

        serde_wasm_bindgen::to_value(&views)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
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
            .map(|descriptor| InputView {
                name: descriptor.kind.name(),
                source: description
                    .inputs
                    .iter()
                    .find(|(wired, _)| *wired == descriptor.kind)
                    .map(|(_, source)| source.index()),
                accepts: descriptor.accepts.iter().map(|kind| kind.as_str()).collect(),
            })
            .collect::<Vec<_>>();

        serde_wasm_bindgen::to_value(&views)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Whether a node is safe to evaluate right now, and why not if it
    /// isn't - reflects the graph's validation state as of the last render/
    /// preview tick (or `Valid` if nothing has ticked yet), not a fresh
    /// re-validation, since recomputing per call would be wasted work the
    /// next tick already does for free.
    pub fn node_validation(&self, node_id: u32) -> Result<JsValue, JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;

        let state = self.graph
            .node_validation(node_id)
            .unwrap_or(NodeValidation::Valid);

        serde_wasm_bindgen::to_value(&NodeValidationView::from(state))
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// The outputs a node declares, labelled (see `Operation::output_names`)
    /// - what a PATCH node's own edit screen offers as candidate outputs
    /// to map, once REFERENCE is wired.
    pub fn node_outputs(&self, node_id: u32) -> Result<JsValue, JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;

        let operation = self.graph
            .get_node(&node_id)
            .ok_or_else(|| {
                JsValue::from_str(
                    &format!("Node {:?} not found", node_id)
                )
            })?;

        let names = operation.output_names();
        let views: Vec<OutputView> = operation
            .metadata()
            .outputs
            .iter()
            .enumerate()
            .map(|(index, kind)| OutputView {
                index: index as u32,
                name: names.get(index).copied().unwrap_or("OUTPUT").to_string(),
                kind: kind.as_str().to_string(),
            })
            .collect();

        serde_wasm_bindgen::to_value(&views)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Which properties a PATCH node's currently-wired SOURCE (target)
    /// can have driven - see `Graph::available_patch_properties`.
    pub fn patch_available_properties(&self, node_id: u32) -> Result<JsValue, JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;
        let properties = self.graph.available_patch_properties(node_id);

        serde_wasm_bindgen::to_value(&properties)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Which output index (if any) a PATCH node currently maps to the
    /// given property, and its combine mode (REPLACE/ADD/SUBTRACT).
    pub fn patch_mapping(&self, node_id: u32, property: String) -> Result<JsValue, JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;

        let node = self.graph
            .resolve(node_id)
            .ok_or_else(|| {
                JsValue::from_str(
                    &format!("Node {:?} not found", node_id)
                )
            })?;

        let mapping = node
            .animation_mappings
            .iter()
            .find(|mapping| mapping.property == property)
            .map(|mapping| PatchMappingView {
                output_index: mapping.output_index as u32,
                mode: patch_mode_name(mapping.mode),
            });

        serde_wasm_bindgen::to_value(&mapping)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Map one of a PATCH node's wired REFERENCE (animation source)'s
    /// outputs to one of its wired SOURCE (target)'s properties, combined
    /// via `mode` ("REPLACE", "ADD", or "SUBTRACT").
    pub fn set_patch_mapping(&mut self, node_id: u32, property: String, output_index: u32, mode: String) -> Result<(), JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;
        let mode = parse_patch_mode(&mode)?;

        self.graph
            .set_patch_mapping(node_id, &property, output_index as usize, mode)
            .map_err(js_err)
    }

    /// Remove one property's mapping, leaving the rest untouched.
    pub fn clear_patch_mapping(&mut self, node_id: u32, property: String) -> Result<(), JsValue> {
        let node_id = resolve_id(&self.graph, node_id)?;

        self.graph
            .clear_patch_mapping(node_id, &property)
            .map_err(js_err)
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

        // Create U8Image from the pixel data
        let image = Arc::new(U8Image {
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
                // Seconds since this App was constructed, so a multi-frame
                // Value::Video advances via Video::frame_at(time) instead of
                // being permanently stuck at time=0.0 (its first frame).
                time: (now_ms() - self.start_time_ms) / 1000.0,
                preview,
                width,
                height,
                ..Meta::default()
            },
            resources: self.resources.clone(),
            input_bboxes: Vec::new(),
            compute: self.compute.clone(),
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

        // Push every PATCH node's current mapped values in - into a real
        // target parameter, or PATCH's own raw-channel state - before
        // the normal DAG walk. See Graph::apply_patch_nodes's own doc
        // comment for why this is a flat pre-pass rather than folded
        // into the executor itself.
        self.graph.apply_patch_nodes(&ctx);

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

        self.output_out_of_gamut = match first_value {
            Value::FloatImage(float_image) => float_image.is_out_of_gamut(),
            _ => false,
        };

        // Renderer boundary dispatch: convert any renderable Value to Frame
        let frame = to_render_frame(first_value)
            .map_err(|e| JsValue::from_str(&format!("Failed to convert to render frame: {:?}", e)))?;

        write_frame_to_canvas(&canvas, &frame)
    }

    /// Whether the node last drawn by render_tick was an out-of-gamut
    /// FloatImage (R/G/B outside 0..1) - the canvas still shows a clamped
    /// approximation of it either way (see ToRenderFrame for FloatImage),
    /// this is what lets the UI tell the user that display isn't the same
    /// thing as an explicit CLAMP.
    pub fn is_output_out_of_gamut(&self) -> bool {
        self.output_out_of_gamut
    }


    pub fn preview_tick(
        &mut self,
        node: usize,
        canvas: HtmlCanvasElement,
    ) -> Result<(), JsValue> {

        self.graph.validate().map_err(js_err)?;

        let node = resolve_id(&self.graph, node as u32)?;

        let ctx = self.context(true);

        // Same pre-pass as render_tick - PREVIEW should show mapped
        // properties too, not just LIVE OUTPUT.
        self.graph.apply_patch_nodes(&ctx);

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
