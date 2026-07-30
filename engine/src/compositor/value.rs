// src/compositor/value.rs

use std::sync::Arc;
use crate::graphics::{
    color::Color,
    frame::Frame,
    geometry::Center,
    image::Image,
    mask::Mask,
    video::Video
};

#[derive(Debug, Clone)]
pub enum Value {
    Frame(Arc<Frame>),
    Mask(Arc<Mask>),
    Image(Arc<Image>),
    Video(Arc<Video>),
    Number(f64),
    Boolean(bool),
    Text(String),
    Color(Color),
    Center(Center)
}

/// Text representation used both for the UI's parameter display and as a
/// cheap fingerprint of an operation's own scalar state (see
/// executors::render's frame-to-frame cache).
pub fn value_to_text(value: &Value) -> String {
    match value {
        Value::Text(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Boolean(flag) => flag.to_string(),
        Value::Color(color) => color.to_hex(),
        other => format!("{:?}", other),
    }
}

/// Whether two Values are the exact same result - Arc pointer identity for
/// pixel-bearing kinds (the only ones ever carried through a wired input),
/// real equality for the plain scalar kinds. Center has no meaningful
/// equality today and is never wired, so it's conservatively never equal
/// (always treated as "changed").
pub fn value_ptr_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Frame(a), Value::Frame(b)) => Arc::ptr_eq(a, b),
        (Value::Mask(a), Value::Mask(b)) => Arc::ptr_eq(a, b),
        (Value::Image(a), Value::Image(b)) => Arc::ptr_eq(a, b),
        (Value::Video(a), Value::Video(b)) => Arc::ptr_eq(a, b),
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Text(a), Value::Text(b)) => a == b,
        (Value::Color(a), Value::Color(b)) => a == b,
        _ => false,
    }
}
