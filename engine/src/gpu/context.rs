// src/gpu/context.rs

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    pub async fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(
                &wgpu::RequestAdapterOptions::default()
            )
            .await
            .map_err(|e| format!("Adapter error: {:?}", e))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor::default()
            )
            .await
            .map_err(|e| format!("Device error: {:?}", e))?;

        Ok(Self {
            device,
            queue,
        })
    }

    pub fn create_shader(
        &self,
        source: &str,
    ) -> wgpu::ShaderModule {

        self.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("compute shader"),
                source: wgpu::ShaderSource::Wgsl(
                    source.into()
                ),
            }
        )
    }
}