// src/compositor/input.rs

use serde::Serialize;

use crate::compositor::value::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Input {
    Source,
    Reference,
    Content,
    Mask,
    Foreground,
    Background,
}

impl Input {
    /// Stable wire name. The UI addresses an operation's inputs by this name
    /// without needing to know anything about the operation itself.
    pub fn name(&self) -> &'static str {
        match self {
            Input::Source => "SOURCE",
            Input::Reference => "REFERENCE",
            Input::Content => "CONTENT",
            Input::Mask => "MASK",
            Input::Foreground => "FOREGROUND",
            Input::Background => "BACKGROUND",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "SOURCE" => Some(Input::Source),
            "REFERENCE" => Some(Input::Reference),
            "CONTENT" => Some(Input::Content),
            "MASK" => Some(Input::Mask),
            "FOREGROUND" => Some(Input::Foreground),
            "BACKGROUND" => Some(Input::Background),
            _ => None,
        }
    }
}

pub fn find_input(inputs: &[(Input, Value)], key: Input) -> Option<&Value> {
    inputs.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
}
