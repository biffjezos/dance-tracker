/**
Stub. Represents a Solid Color plane. It appears in the UI under NODES, once added.
**/

use crate::compositor::Color;
use wasm_bindgen::{ JsValue };

pub struct SolidColor {
    pub color: Color,
}

impl SolidColor {
    pub fn new(color: Color) -> Result<SolidColor, JsValue> {

    }
}