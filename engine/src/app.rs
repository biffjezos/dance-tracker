#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, HtmlVideoElement};

use crate::compositor::{
    context::{ Context, Meta },
    error::OperationError,
    graph::{Graph, NodeId},
};

use crate::compositor::executor::{PreviewExecutor, RenderExecutor, SimpleExecutor};
use crate::dom::write_frame_to_canvas;
use crate::graphics::Frame;
use crate::resources::ResourceManager;

fn js_err(err: OperationError) -> JsValue {
    JsValue::from_str(&format!("{:?}", err))
}

#[wasm_bindgen]
pub struct App {
    graph: Graph,
    resources: ResourceManager,
    frame_counter: u64,
}


#[wasm_bindgen]
impl App {

    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> App {
        App {
            graph: Graph::new(width, height),
            resources: ResourceManager::new(),
            frame_counter: 0,
        }
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


    pub fn play(
        &self,
        video: HtmlVideoElement,
    ) -> Result<(), JsValue> {
        video
            .play()
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