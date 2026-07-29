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
