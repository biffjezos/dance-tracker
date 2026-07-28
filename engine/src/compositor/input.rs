// src/compositor/input.rs

use crate::compositor::value::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Source,
    Reference,
    Content,
    Mask,
    Foreground,
    Background,
}

pub fn find_input(inputs: &[(Input, Value)], key: Input) -> Option<&Value> {
    inputs.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
}