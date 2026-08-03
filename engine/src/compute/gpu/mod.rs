pub mod blur;

use crate::compute::backend::ComputeBackend;
use crate::gpu::context::GpuContext;

pub struct GpuBackend {
    pub blur: blur::GpuBlur,
}

impl GpuBackend {
    pub async fn new() -> Result<Self, String> {
        let gpu = GpuContext::new().await?;

        Ok(Self {
            blur: blur::GpuBlur::new(gpu),
        })
    }
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