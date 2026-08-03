use crate::compute::backend::ComputeBackend;
use crate::gpu::context::GpuContext;
use crate::gpu::BLUR_SHADER;

pub struct GpuBlur {
    pub gpu: GpuContext,
    pub pipeline: wgpu::ComputePipeline,
}

impl GpuBlur {
    pub fn new(gpu: GpuContext) -> Self {

        let shader =
            gpu.create_shader(BLUR_SHADER);

        let pipeline =
            gpu.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("blur pipeline"),
                    layout: None,
                    module: &shader,
                    entry_point: Some("main"),
                    cache: None,
                    compilation_options:
                        wgpu::PipelineCompilationOptions::default(),
                }
            );

        Self {
            gpu,
            pipeline,
        }
    }
}

impl ComputeBackend for GpuBlur {
    fn blur( &self, pixels: &[f32], _width: u32, _height: u32, _radius: u32, ) -> Vec<f32> {
        pixels.to_vec()
    }
}