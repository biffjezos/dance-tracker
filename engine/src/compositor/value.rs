use std::sync::Arc;
use crate::graphics::{Center, Color, Frame, Image, Mask }
/*
Clone is cheap for every variant that matters: Frame/Mask/Image clone
an Arc (a refcount bump, never pixel data), Number/Boolean are Copy,
Text clones a (typically short) String - this is what the Arc move in
the Value enum rewrite was for, and RenderExecutor's per-tick
memoization (a shared node's Value handed to N consumers) is the
concrete case that needs it.
*/
#[derive(Debug, Clone)]
pub enum Value {
    Frame(Arc<Frame>),
    Mask(Arc<Mask>),
    Image(Arc<Image>),
    Number(f64),
    Boolean(bool),
    Text(String),
    Color(Color),
    Center(Center)
}
