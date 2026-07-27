/*
Stub - just enough to compile. Represents a solid colour plane;
appears in the UI under NODES once added. Not yet a real Operation
(no execute()/metadata() impl) - that lands with the rest of the
Color/SolidColor work.
*/

use crate::compositor::Color;
use wasm_bindgen::JsValue;

pub struct SolidColor {
    pub color: Color,
}

impl SolidColor {
    pub fn new(color: Color) -> Result<SolidColor, JsValue> {
        Ok(SolidColor { color })
    }
}