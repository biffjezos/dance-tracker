/*
Where a source operation's pixels actually come from. Confirmed via
dom.rs (compiles for real against wasm32-unknown-unknown): Rust can
read a canvas's current pixels, and can draw a live <video> element's
current frame onto a canvas itself, both through web-sys - so a source
operation reaches into the DOM directly, no JS pixel-pushing per tick
needed. PixelSource decouples "what pixels does this source have right
now" from "how were they obtained," so VideoSource stays unit-testable
with a fixed in-memory Frame instead of needing a real browser.
*/

use crate::compositor::OperationError;
use crate::operations::Frame;

pub trait PixelSource {
    fn read(&self) -> Result<Frame, OperationError>;
}

pub mod video;
pub mod captured;

pub use captured::CapturedFrame;
