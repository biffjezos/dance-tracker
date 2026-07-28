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
    OperationError,
    OperationRegistry
};
use crate::dom::write_frame_to_canvas;
use crate::graphics::Frame;
use crate::resources::manager::ResourceManager;

fn js_err(err: OperationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}

fn expect_frame(
    value: Option<&crate::compositor::Value>,
) -> Result<&Frame, OperationError> {
    match value {
        Some(crate::compositor::Value::Frame(frame)) => Ok(frame.as_ref()),
        _ => Err(OperationError::WrongValueType),
    }
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
    #[wasm_bindgen]
    pub fn get_operations(&self) -> JsValue {
        serde_wasm_bindgen::to_value(
            &self.registry.descriptors()
        )
        .unwrap()
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

        let frame = expect_frame(values.first()).map_err(js_err)?;

        write_frame_to_canvas(&canvas, frame)
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

        let frame = expect_frame(values.first()).map_err(js_err)?;

        write_frame_to_canvas(&canvas, frame)
    }


    pub fn play( &self, video: HtmlVideoElement, ) -> Result<(), JsValue> {
        video
            .play()
            .map(|_| ())
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }


    pub fn stop(
        &self,
        video: HtmlVideoElement,
    ) -> Result<(), JsValue> {
        video
            .pause()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    }


    pub fn forward(
        &self,
        video: HtmlVideoElement,
        seconds: f64,
    ) -> Result<(), JsValue> {
        video.set_current_time(video.current_time() + seconds);
        Ok(())
    }


    pub fn rewind(
        &self,
        video: HtmlVideoElement,
        seconds: f64,
    ) -> Result<(), JsValue> {
        video.set_current_time(video.current_time() - seconds);
        Ok(())
    }
}