// src/compute/mod.rs
pub mod backend;
pub mod cpu;
pub mod gpu;
pub mod params;
pub use gpu::create_backend;

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