// src/compute/cpu/mod.rs

use crate::compute::backend::ComputeBackend;

pub struct CpuBackend;

impl ComputeBackend for CpuBackend {
    fn blur(
        &self,
        pixels: &[f32],
        width: u32,
        height: u32,
        radius: u32,
    ) -> Vec<f32> {

        crate::operations::transform::blur::blur_pixels_static(
            pixels,
            width,
            height,
            radius,
        )
    }
}