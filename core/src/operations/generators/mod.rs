pub mod ghost;
pub mod rings;
pub mod text;

pub use solid::SolidColour;
pub use ghost::Ghost;

#[cfg(target_arch = "wasm32")]
pub use rings::Rings;

#[cfg(target_arch = "wasm32")]
pub use text::Text;
