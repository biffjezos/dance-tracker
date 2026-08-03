use crate::compute::backend::ComputeBackend;
use crate::gpu::context::GpuContext;

pub struct GpuBlur {
    pub gpu: GpuContext,
}

impl GpuBlur {
    pub fn new(gpu: GpuContext) -> Self {
        Self {
            gpu,
        }
    }
}

impl ComputeBackend for GpuBlur {
    fn blur(
        &self,
        pixels: &[f32],
        _width: u32,
        _height: u32,
        _radius: u32,
    ) -> Vec<f32> {
        pixels.to_vec()
    }
}