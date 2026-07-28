// src/dom.rs

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlVideoElement, ImageData};

use crate::compositor::OperationError;
use crate::operations::sources::PixelSource;
use crate::operations::Frame;

fn to_js_error(err: OperationError, context: &str) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(&format!("{}: {:?}", context, err))
}

fn canvas_2d(canvas: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d, OperationError> {
    canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|ctx| ctx.dyn_into::<CanvasRenderingContext2d>().ok())
        .ok_or(OperationError::WrongValueType)
}

fn contain_fit(source_w: f64, source_h: f64, target_w: f64, target_h: f64) -> (f64, f64, f64, f64) {
    if source_w <= 0.0 || source_h <= 0.0 {
        return (0.0, 0.0, target_w, target_h);
    }

    let scale = (target_w / source_w).min(target_h / source_h);
    let width = source_w * scale;
    let height = source_h * scale;

    ((target_w - width) / 2.0, (target_h - height) / 2.0, width, height)
}

pub struct VideoElementPixelSource {
    pub video: HtmlVideoElement,
    pub scratch_canvas: HtmlCanvasElement,
}

impl PixelSource for VideoElementPixelSource {
    fn read(&self, width: u32, height: u32) -> Result<Frame, OperationError> {
        self.scratch_canvas.set_width(width);
        self.scratch_canvas.set_height(height);

        let ctx = canvas_2d(&self.scratch_canvas)?;

        ctx.clear_rect(0.0, 0.0, width as f64, height as f64);

        let (x, y, w, h) = contain_fit(
            self.video.video_width() as f64,
            self.video.video_height() as f64,
            width as f64,
            height as f64,
        );

        ctx.draw_image_with_html_video_element_and_dw_and_dh(&self.video, x, y, w, h)
            .map_err(|_| OperationError::WrongValueType)?;

        let image_data: ImageData = ctx
            .get_image_data(0.0, 0.0, width as f64, height as f64)
            .map_err(|_| OperationError::WrongValueType)?;

        Ok(Frame {
            pixels: image_data.data().0,
            width,
            height,
            timestamp: self.video.current_time(),
        })
    }
}

pub fn write_frame_to_canvas(
    canvas: &HtmlCanvasElement,
    frame: &Frame,
) -> Result<(), wasm_bindgen::JsValue> {
    canvas.set_width(frame.width);
    canvas.set_height(frame.height);

    let ctx = canvas_2d(canvas).map_err(|e| to_js_error(e, "write_frame_to_canvas"))?;

    let mut pixels = frame.pixels.clone();

    let image_data = ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&mut pixels),
        frame.width,
        frame.height,
    )?;

    ctx.put_image_data(&image_data, 0.0, 0.0)
}
