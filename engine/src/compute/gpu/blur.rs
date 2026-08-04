// src/compute/gpu/blur.rs

use crate::compute::backend::ComputeBackend;
use crate::compute::params::BlurParams;
use crate::gpu::context::GpuContext;
use crate::gpu::BLUR_SHADER;

use bytemuck;
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
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
                    compilation_options: Default::default(),
                    cache: None,
                }
            );


        Self {
            gpu,
            pipeline,
            bind_group_layout,
        }
    }


    fn read_buffer(
        &self,
        buffer: &wgpu::Buffer,
        size: usize,
    ) -> Vec<f32> {

        let slice = buffer.slice(..);

        let (sender, receiver) =
            std::sync::mpsc::channel();


        slice.map_async(
            wgpu::MapMode::Read,
            move |result| {
                let _ = sender.send(result);
            },
        );


        self.gpu.device.poll(
            wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            }
        );


        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                panic!(
                    "GPU buffer mapping failed: {:?}",
                    error
                );
            }
            Err(error) => {
                panic!(
                    "GPU mapping channel failed: {}",
                    error
                );
            }
        }

        let data = slice.get_mapped_range().expect("Failed to map GPU buffer");
        let result: &[f32] = bytemuck::cast_slice(&data);
        let result = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        buffer.unmap();
        result[..size].to_vec()
    }
}

// src/compute/gpu/blur.rs
// continuation

impl ComputeBackend for GpuBlur {

    fn blur(
        &self,
        pixels: &[f32],
        width: u32,
        height: u32,
        radius: u32,
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
                        (pixels.len()
                        * std::mem::size_of::<f32>())
                        as u64,
                    usage:
                        wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }
            );


        let readback_buffer =
            self.gpu.device.create_buffer(
                &wgpu::BufferDescriptor {
                    label: Some("blur readback"),
                    size:
                        (pixels.len()
                        * std::mem::size_of::<f32>())
                        as u64,
                    usage:
                        wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }
            );


        let params =
            BlurParams {
                width,
                height,
                radius,
                _padding: 0,
            };


        let params_buffer =
            self.gpu.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("blur params"),
                    contents:
                        bytemuck::bytes_of(&params),
                    usage:
                        wgpu::BufferUsages::UNIFORM,
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
                                input_buffer
                                .as_entire_binding(),
                        },

                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource:
                                output_buffer
                                .as_entire_binding(),
                        },

                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource:
                                params_buffer
                                .as_entire_binding(),
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


        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &readback_buffer,
            0,
            (pixels.len()
            * std::mem::size_of::<f32>())
            as u64,
        );


        self.gpu.queue.submit(
            Some(encoder.finish())
        );


        self.read_buffer(
            &readback_buffer,
            pixels.len(),
        )
    }
}