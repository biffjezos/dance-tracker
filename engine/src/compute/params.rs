// src/compute/params.rs
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlurParams {
    width: u32,
    height: u32,
    radius: u32,
    _padding: u32,
}