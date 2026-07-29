// src/app.rs
#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlVideoElement};

use crate::compositor::{
    Context,
    executors::{
        Execute,
        PreviewExecutor,
        RenderExecutor,
        SimpleExecutor
    },
    graph::{ Graph, NodeId },
    Meta,
    Operation,
    OperationError,
    OperationRegistry,
    Value
};
use crate::graphics::Video;
use crate::dom::{ VideoElementPixelSource, write_frame_to_canvas};
use crate::operations::sources::{ImageSource, VideoSource};
use crate::operations::transform::Shuffle;

use crate::renderer::to_render_frame;
use crate::resources::manager::ResourceManager;
use std::sync::Arc;

fn js_err(err: OperationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}

#[wasm_bindgen]
pub struct App {
    graph: Graph,
    resources: ResourceManager,
    registry: OperationRegistry,
    frame_counter: u64,
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
    
    /// Create an image source node and return its ID
    pub fn create_image_source_node(&mut self) -> Result<u32, JsValue> {
        let operation = Box::new(ImageSource::new());
        let node_id = self.graph.add_node(operation);

    /// Create a video source node and return its ID
    pub fn create_video_source_node(&mut self) -> Result<u32, JsValue> {
        let operation = Box::new(VideoSource::new());
        let node_id = self.graph.add_node(operation);

        Ok(node_id.index())
    }
    
    /// Check if a node supports editing (has editable parameters)
    pub fn node_supports_edit(&self, node_id: u32) -> bool {
        let node_id = NodeId::from_index(node_id);
        self.graph.get_node(&node_id)
            .map(|op| op.supports_edit())
            .unwrap_or(false)
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
        let node_id = NodeId::from_index(node_id);
        
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

    /// Set image data on a specific VideoSource node
    /// Takes pixel data as Uint8Array, width, height
    pub fn set_video_element_on_node(
        &mut self,
        node_id: u32,
        video: HtmlVideoElement,
        scratch_canvas: HtmlCanvasElement,
    ) -> Result<(), JsValue> {

        let node_id = NodeId::from_index(node_id);

        let operation = self.graph
            .get_node_mut(&node_id)
            .ok_or_else(|| {
                JsValue::from_str(
                    &format!("Node {:?} not found", node_id)
                )
            })?;

        let video_source = operation
            .as_any_mut()
            .downcast_mut::<VideoSource>()
            .ok_or_else(|| {
                JsValue::from_str(
                    "Node is not a VideoSource"
                )
            })?;

        let pixel_source = VideoElementPixelSource {
            video,
            scratch_canvas,
        };

        let video = Video::new(
            Arc::new(pixel_source)
        );

        video_source.set_video(
            Arc::new(video)
        );

        Ok(())
    }
    
    /// Update a parameter on a specific node
    /// Takes node_id, parameter name, and value as string
    pub fn update_node_parameter(
        &mut self,
        node_id: u32,
        parameter: String,
        value: String,
    ) -> Result<(), JsValue> {
        let node_id = NodeId::from_index(node_id);
        
        // Get the operation from the graph
        let operation = self.graph.get_node_mut(&node_id)
            .ok_or_else(|| JsValue::from_str(&format!("Node {:?} not found", node_id)))?;
        
        // Try to downcast to Shuffle and update parameter
        if let Some(shuffle) = operation.as_any_mut().downcast_mut::<Shuffle>() {
            match parameter.as_str() {
                "red_channel" => {
                    shuffle.red = parse_shuffle_channel(&value)
                        .ok_or_else(|| JsValue::from_str(&format!("Invalid channel value: {}", value)))?;
                    return Ok(());
                }
                "green_channel" => {
                    shuffle.green = parse_shuffle_channel(&value)
                        .ok_or_else(|| JsValue::from_str(&format!("Invalid channel value: {}", value)))?;
                    return Ok(());
                }
                "blue_channel" => {
                    shuffle.blue = parse_shuffle_channel(&value)
                        .ok_or_else(|| JsValue::from_str(&format!("Invalid channel value: {}", value)))?;
                    return Ok(());
                }
                "alpha_channel" => {
                    shuffle.alpha = parse_shuffle_channel(&value)
                        .ok_or_else(|| JsValue::from_str(&format!("Invalid channel value: {}", value)))?;
                    return Ok(());
                }
                _ => return Err(JsValue::from_str(&format!("Unknown parameter: {}", parameter))),
            };
        }
        
        Err(JsValue::from_str(&format!("Node {:?} does not support parameter updates", node_id)))
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

        let ctx = self.context(false);

        let executor = RenderExecutor;

        let values = executor
            .execute(
                &self.graph,
                NodeId::from_index(output_node as u32),
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

        let ctx = self.context(true);

        let executor = PreviewExecutor;

        let values = executor
            .execute(
                &self.graph,
                NodeId::from_index(node as u32),
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

/// Parse a string into ShuffleChannel
fn parse_shuffle_channel(s: &str) -> Option<crate::operations::transform::shuffle::ShuffleChannel> {
    use crate::operations::transform::shuffle::ShuffleChannel;
    match s.to_lowercase().as_str() {
        "red" => Some(ShuffleChannel::R),
        "r" => Some(ShuffleChannel::R),
        "green" => Some(ShuffleChannel::G),
        "g" => Some(ShuffleChannel::G),
        "blue" => Some(ShuffleChannel::B),
        "b" => Some(ShuffleChannel::B),
        "alpha" => Some(ShuffleChannel::A),
        "a" => Some(ShuffleChannel::A),
        "off" => Some(ShuffleChannel::Off),
        _ => None,
    }
}
