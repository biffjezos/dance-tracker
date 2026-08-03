// src/compute/gpu/mod.rs
pub mod blur;

use crate::compute::backend::ComputeBackend;

pub struct GpuBackend {
    pub blur: blur::GpuBlur,
}

impl ComputeBackend for GpuBackend {

    fn blur(
        &self,
        pixels: &[f32],
        width: u32,
        height: u32,
        radius: u32,
    ) -> Vec<f32> {

        self.blur.blur(
            pixels,
            width,
            height,
            radius,
        )
    }
}