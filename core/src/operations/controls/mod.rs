/*
Controls are one-off commands against a <video> element (play/pause/
seek), not per-frame frame transforms - run through SimpleExecutor when
the user clicks a TRANSPORT button, not evaluated every tick by
Preview/RenderExecutor. wasm32-only: there's nothing to control
natively.
*/

pub mod play;
pub mod stop;
pub mod forward;
pub mod rewind;

#[cfg(target_arch = "wasm32")]
pub use play::Play;

#[cfg(target_arch = "wasm32")]
pub use stop::Stop;

#[cfg(target_arch = "wasm32")]
pub use forward::Forward;

#[cfg(target_arch = "wasm32")]
pub use rewind::Rewind;
