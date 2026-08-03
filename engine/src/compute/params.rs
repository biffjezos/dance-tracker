// src/compute/params.rs
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlurParams {
    pub width: u32,
    pub height: u32,
    pub radius: u32,
    pub _padding: u32,
}