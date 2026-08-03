use crate::compute::backend::ComputeBackend;
use crate::gpu::context::GpuContext;
use crate::gpu::BLUR_SHADER;
use wgpu::util::DeviceExt;
use bytemuck;

pub struct GpuBlur {
    pub gpu: GpuContext,
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}
struct GpuOutput {
    buffer: wgpu::Buffer,
    size: usize,
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

        let pipeline_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("blur pipeline layout"),
                bind_group_layouts: &[
                    Some(&bind_group_layout)
                ],
                immediate_size: 0,
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
    fn read_buffer(&self, buffer: &wgpu::Buffer, size: usize, ) -> Vec<f32> {

        let slice = buffer.slice(..);
        let (sender, receiver) =vstd::sync::mpsc::channel();

        slice.map_async(
            wgpu::MapMode::Read,
            move |result| {
                sender.send(result).unwrap();
            },
        );

        self.gpu.device.poll(
            wgpu::PollType::Wait
        ).unwrap();

        receiver.recv().unwrap().unwrap();

        let data = slice.get_mapped_range();

        let result = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        buffer.unmap();
        result[..size].to_vec()
    }
}

impl ComputeBackend for GpuBlur {

    fn blur(&self,
        pixels: &[f32],
        width: u32,
        height: u32,
        _radius: u32,
    ) -> Vec<f32> {

        let input_buffer =
            self.gpu.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("blur input"),
                    contents: bytemuck::cast_slice(pixels),
                    usage:
                        wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC,
                }
            );


        let output_buffer =
            self.gpu.device.create_buffer(
                &wgpu::BufferDescriptor {
                    label: Some("blur output"),
                    size:
                        (pixels.len() * 4) as u64,
                    usage:
                        wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }
            );


        let bind_group =
            self.gpu.device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    label: Some("blur bind group"),
                    layout:
                        &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource:
                                input_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource:
                                output_buffer.as_entire_binding(),
                        },
                    ],
                }
            );


        let mut encoder =
            self.gpu.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor {
                    label: Some("blur encoder"),
                }
            );


        {
            let mut pass =
                encoder.begin_compute_pass(
                    &wgpu::ComputePassDescriptor {
                        label: Some("blur pass"),
                        timestamp_writes: None,
                    }
                );

            pass.set_pipeline(
                &self.pipeline
            );

            pass.set_bind_group(
                0,
                &bind_group,
                &[],
            );

            pass.dispatch_workgroups(
                (width + 15) / 16,
                (height + 15) / 16,
                1,
            );
        }


        self.gpu.queue.submit(
            Some(encoder.finish())
        );


        self.read_buffer(&output_buffer, pixels.len(), )
    }
}