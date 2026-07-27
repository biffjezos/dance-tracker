/*
Same reasoning as rings.rs: text layout/shaping is what Canvas2D's own
fillText/measureText already do well - reimplementing font rasterization
in pure Rust is out of scope for "get it working like before." wasm32
only, drawn via web-sys onto a private detached scratch canvas.
*/
#![cfg(target_arch = "wasm32")]

use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::compositor::{
    Context, Input, Operation, OperationCategory, OperationError, OperationMetadata, OutputKind,
    ParameterDescriptor, ParameterKind, Value,
};
use crate::operations::Frame;

pub struct Text {
    pub content: String,
    pub colour: String,
    pub size: f64,

    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
}

impl Text {
    pub fn new(content: String, colour: String, size: f64) -> Result<Text, JsValue> {
        let document = web_sys::window()
            .expect("window should exist")
            .document()
            .expect("document should exist");

        // Sized on the first execute() call, from Context::meta, not
        // here - see execute() below.
        let canvas: HtmlCanvasElement = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;

        let ctx = canvas
            .get_context("2d")?
            .expect("canvas should have a 2d context")
            .dyn_into::<CanvasRenderingContext2d>()?;

        Ok(Text { content, colour, size, canvas, ctx })
    }

    fn wrap_lines(&self, raw_text: &str, max_width: f64) -> Result<Vec<String>, JsValue> {
        let mut lines = Vec::new();

        for paragraph in raw_text.split(['\n']) {
            let words: Vec<&str> = paragraph.split(' ').filter(|w| !w.is_empty()).collect();

            if words.is_empty() {
                lines.push(String::new());
                continue;
            }

            let mut current_line = words[0].to_string();

            for word in &words[1..] {
                let test_line = format!("{} {}", current_line, word);

                let width = self.ctx.measure_text(&test_line)?.width();

                if width > max_width {
                    lines.push(current_line);
                    current_line = word.to_string();
                } else {
                    current_line = test_line;
                }
            }

            lines.push(current_line);
        }

        Ok(lines)
    }
}

impl Operation for Text {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Text",
            category: OperationCategory::Generator,
            input_count: 0,
            outputs: vec![OutputKind::Frame],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor { name: "content", kind: ParameterKind::Text },
            ParameterDescriptor { name: "colour", kind: ParameterKind::Text },
            ParameterDescriptor { name: "size", kind: ParameterKind::Number },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "content" => Some(Value::Text(self.content.clone())),
            "colour" => Some(Value::Text(self.colour.clone())),
            "size" => Some(Value::Number(self.size)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("content", Value::Text(v)) => { self.content = v; Ok(()) }
            ("colour", Value::Text(v)) => { self.colour = v; Ok(()) }
            ("size", Value::Number(v)) => { self.size = v; Ok(()) }
            ("content" | "colour" | "size", _) => Err(OperationError::WrongValueType),
            _ => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    fn execute(
        &self,
        ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        self.canvas.set_width(ctx.meta.width);
        self.canvas.set_height(ctx.meta.height);

        let width = self.canvas.width();
        let height = self.canvas.height();

        self.ctx.clear_rect(0.0, 0.0, width as f64, height as f64);

        let content = self.content.trim();

        if content.is_empty() {
            let image_data = self
                .ctx
                .get_image_data(0.0, 0.0, width as f64, height as f64)
                .map_err(|_| OperationError::WrongValueType)?;

            return Ok(vec![Value::Frame(std::sync::Arc::new(Frame {
                pixels: image_data.data().0,
                width,
                height,
                timestamp: 0.0,
            }))]);
        }

        self.ctx.save();

        self.ctx.set_font(&format!("bold {}px Arial, sans-serif", self.size));
        self.ctx.set_fill_style_str(&self.colour);
        self.ctx.set_text_align("center");
        self.ctx.set_text_baseline("middle");

        let max_width = width as f64 - 20.0;

        let lines = self
            .wrap_lines(&self.content, max_width)
            .map_err(|_| OperationError::WrongValueType)?;

        let line_height = self.size * 1.15;
        let total_height = lines.len() as f64 * line_height;

        let mut y = (height as f64 - total_height) / 2.0 + line_height / 2.0;

        for line in &lines {
            self.ctx
                .fill_text(line, width as f64 / 2.0, y)
                .map_err(|_| OperationError::WrongValueType)?;

            y += line_height;
        }

        self.ctx.restore();

        let image_data = self
            .ctx
            .get_image_data(0.0, 0.0, width as f64, height as f64)
            .map_err(|_| OperationError::WrongValueType)?;

        Ok(vec![Value::Frame(std::sync::Arc::new(Frame {
            pixels: image_data.data().0,
            width,
            height,
            timestamp: 0.0,
        }))])
    }
}
