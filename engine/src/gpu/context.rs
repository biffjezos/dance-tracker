pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::default();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default() )
            .await
            .unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default() )
            .await
            .unwrap();

        Self {
            device,
            queue,
        }
    }
}

impl GpuContext {
    pub fn create_shader( &self, source: &str,) -> wgpu::ShaderModule {
        self.device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("compute shader"),
                source: wgpu::ShaderSource::Wgsl( source.into() ),
            }
        )
    }
}