pub mod backend;
pub mod cpu;
pub mod gpu;
pub mod params;

use std::sync::Arc;

use crate::compositor::ComputeMode;
use crate::compute::backend::ComputeBackend;
use crate::compute::cpu::CpuBackend;
use crate::compute::gpu::GpuBackend;

pub fn create_backend(mode: ComputeMode) -> Arc<dyn ComputeBackend> {
    match mode {
        ComputeMode::Cpu => {
            Arc::new(CpuBackend)
        }

        ComputeMode::Gpu => {
            Arc::new(
                GpuBackend::new()
                    .expect("Failed to initialize GPU backend")
            )
        }

        ComputeMode::Auto => {
            match GpuBackend::new() {
                Ok(gpu) => Arc::new(gpu),
                Err(_) => Arc::new(CpuBackend),
            }
        }
    }
}