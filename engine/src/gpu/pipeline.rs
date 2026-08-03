use crate::gpu::context::GpuContext;
use crate::gpu::BLUR_SHADER;

pub struct BlurPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl BlurPipeline {
    pub fn new(context: &GpuContext) -> Self {
        let shader = context.create_shader(BLUR_SHADER);

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(
                    &wgpu::BindGroupLayoutDescriptor {
                        label: Some("blur bind group layout"),
                        entries: &[],
                    },
                );

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("blur pipeline layout"),
                        bind_group_layouts: &[&bind_group_layout],
                        push_constant_ranges: &[],
                    },
                );

        let pipeline =
            context
                .device
                .create_compute_pipeline(
                    &wgpu::ComputePipelineDescriptor {
                        label: Some("blur pipeline"),
                        layout: Some(&pipeline_layout),
                        module: &shader,
                        entry_point: Some("main"),
                        compilation_options: Default::default(),
                        cache: None,
                    },
                );

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    pub fn pipeline(&self) -> &wgpu::ComputePipeline {
        &self.pipeline
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}