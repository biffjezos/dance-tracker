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

pub fn create_backend(mode: ComputeMode) -> Arc<dyn ComputeBackend> {
    match mode {
        ComputeMode::Cpu => Arc::new(CpuBackend),

        ComputeMode::Gpu => Arc::new(
            GpuBackend::new()
                .expect("Failed to initialize GPU backend")
        ),

        ComputeMode::Auto => {
            match GpuBackend::new() {
                Ok(gpu) => Arc::new(gpu),
                Err(_) => Arc::new(CpuBackend),
            }
        }
    }
}