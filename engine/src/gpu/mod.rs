// src/gpu/mod.rs
pub mod buffer;
pub mod context;
pub mod pipeline;
pub const BLUR_SHADER: &str = include_str!("shaders/blur.wgsl");