/*
Real DOM-backed PixelSource implementations - wasm32 only, since these
call web-sys, which needs an actual browser/JS environment underneath.
Confirmed compiling against real web-sys signatures (not assumed):
Rust can draw a live <video> element's current frame onto a canvas via
draw_image_with_html_video_element, then read that canvas's pixels
back via get_image_data - a source operation does both itself, once
per tick, with no JS pixel-pushing involved.
*/

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

/*
A video/camera source: draws the live <video> element's current frame
onto its own scratch canvas, then reads that canvas's pixels back -
the canvas here is Rust's own working buffer, not something JS has to
keep refreshing.
*/
pub struct VideoElementPixelSource {
    pub video: HtmlVideoElement,
    pub scratch_canvas: HtmlCanvasElement,
}

impl PixelSource for VideoElementPixelSource {
    fn read(&self) -> Result<Frame, OperationError> {
        let width = self.video.video_width();
        let height = self.video.video_height();

        if width == 0 || height == 0 {
            return Err(OperationError::WrongValueType);
        }

        self.scratch_canvas.set_width(width);
        self.scratch_canvas.set_height(height);

        let ctx = canvas_2d(&self.scratch_canvas)?;

        ctx.draw_image_with_html_video_element(&self.video, 0.0, 0.0)
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

/*
Writes a composited Frame back onto the visible output canvas - the
last step PreviewExecutor/RenderExecutor takes each tick.
*/
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
