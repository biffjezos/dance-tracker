use crate::compute::backend::ComputeBackend;
use crate::gpu::context::GpuContext;
use crate::gpu::BLUR_SHADER;
use wgpu::util::DeviceExt;

pub struct GpuBlur {
    pub gpu: GpuContext,
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuBlur {
    pub fn new(gpu: GpuContext) -> Self {
        let shader = gpu.create_shader(BLUR_SHADER);

        let bind_group_layout =
            gpu.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("blur layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage {
                                    read_only: true,
                                },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage {
                                    read_only: false,
                                },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                }
            );

        let pipeline_layout =
            gpu.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("blur pipeline layout"),
                    bind_group_layouts: &[
                        &bind_group_layout
                    ],
                    push_constant_ranges: &[],
                }
            );

        let pipeline =
            gpu.device.create_compute_pipeline(
                &wgpu::ComputePipelineDescriptor {
                    label: Some("blur pipeline"),
                    layout: Some(&pipeline_layout),
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
            bind_group_layout,
        }
    }
}