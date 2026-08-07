// src/gpu/mod.rs
//
// The reusable boundary between the app and WebGPU/wgpu - see
// SPECwebgpucomputebackend2.md's Phase 0. Everything that knows about
// adapter/device/queue setup, or wgpu type specifics, lives here, once,
// so a GPU-capable operation (specified separately, per operation, in
// SPEC-webgpu-operations.md) never has to touch that setup itself - it
// only ever calls these small, generic, operation-agnostic helpers.
//
// Deliberately has NO operation-specific method (no `GpuState::blur()`
// or similar) - that was the removed attempt's core mistake (see
// RFC-001). Each operation owns its own shader, pipeline, and dispatch
// logic on top of these primitives.
//
// Readback is the one place native and wasm32 genuinely diverge, because
// WebGPU buffer mapping is unavoidably asynchronous in a browser - there
// is no blocking wait available there at all, unlike native backends
// (Vulkan/Metal/DX12), which do support blocking on the GPU. This split
// follows the exact precedent already in this codebase
// (`profiling::measure_ms`'s `#[cfg(target_arch = "wasm32")]` split) -
// see `read_buffer_blocking` (native) vs. `read_buffer_async` (wasm32)
// below. No other function in this module blocks on either target.

use wgpu::util::DeviceExt;

pub struct GpuState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuState {
    /// Requests an adapter/device/queue. Resolves to `Err` - never
    /// panics - on a machine with no WebGPU/wgpu-compatible GPU at all,
    /// or any other adapter/device request failure. That `Err` is the
    /// expected, ordinary "GPU not available" outcome: see
    /// `Context.gpu`'s own doc comment for how the caller (`App::init_gpu`)
    /// turns this into a plain `None` rather than surfacing an error to
    /// the user.
    pub async fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .map_err(|error| format!("no compatible GPU adapter: {:?}", error))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|error| format!("failed to request a GPU device: {:?}", error))?;

        Ok(Self { device, queue })
    }

    pub fn create_shader(&self, wgsl: &str) -> wgpu::ShaderModule {
        self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu compute shader"),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        })
    }

    /// Builds a compute pipeline from a shader + its bind group layouts.
    /// Entirely generic over what the shader actually does - the caller
    /// (a specific operation) owns the shader source, the layout shape,
    /// and the entry point name.
    pub fn create_compute_pipeline(
        &self,
        label: &str,
        shader: &wgpu::ShaderModule,
        entry_point: &str,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::ComputePipeline {
        let layouts: Vec<Option<&wgpu::BindGroupLayout>> =
            bind_group_layouts.iter().map(|layout| Some(*layout)).collect();

        let pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &layouts,
            immediate_size: 0,
        });

        self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: Some(entry_point),
            compilation_options: Default::default(),
            cache: None,
        })
    }

    /// Uploads `data` into a new GPU buffer with the given usage flags -
    /// e.g. a STORAGE|COPY_SRC input buffer, or a UNIFORM params buffer.
    pub fn upload<T: bytemuck::Pod>(&self, label: &str, data: &[T], usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage,
        })
    }

    /// An empty GPU buffer of `size` bytes - a compute pass's output
    /// buffer, or a CPU-readable readback buffer, typically.
    pub fn create_buffer(&self, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        })
    }

    pub fn create_bind_group(
        &self,
        label: &str,
        layout: &wgpu::BindGroupLayout,
        entries: &[wgpu::BindGroupEntry],
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries,
        })
    }

    /// Encodes and submits a single compute dispatch. Does not read
    /// anything back - see `copy_buffer_to_buffer` +
    /// `read_buffer_blocking`/`read_buffer_async` for that, kept
    /// separate since not every dispatch needs an immediate readback.
    pub fn dispatch(
        &self,
        label: &str,
        pipeline: &wgpu::ComputePipeline,
        bind_group: &wgpu::BindGroup,
        workgroups: (u32, u32, u32),
    ) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None,
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }

        self.queue.submit(Some(encoder.finish()));
    }

    /// Copies a GPU-only (STORAGE) output buffer into a MAP_READ-capable
    /// readback buffer - the required intermediate step before either
    /// readback path below, since a STORAGE buffer can't be mapped
    /// directly.
    pub fn copy_buffer_to_buffer(&self, source: &wgpu::Buffer, destination: &wgpu::Buffer, size: u64) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu buffer copy"),
        });
        encoder.copy_buffer_to_buffer(source, 0, destination, 0, size);
        self.queue.submit(Some(encoder.finish()));
    }

    /// Blocking readback - native only. Native wgpu backends
    /// (Vulkan/Metal/DX12) genuinely support blocking on the GPU, unlike
    /// WebGPU in a browser, so this is correct here and *only* here. Any
    /// GPU-backed operation's `#[cfg(not(target_arch = "wasm32"))]`
    /// branch uses this - see SPECwebgpucomputebackend2.md's "Target-
    /// conditional dispatch, not two designs".
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_buffer_blocking(&self, buffer: &wgpu::Buffer, len: usize) -> Vec<f32> {
        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();

        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });

        receiver
            .recv()
            .expect("gpu mapping channel closed before a result arrived")
            .expect("gpu buffer mapping failed");

        let data = slice.get_mapped_range().expect("gpu buffer mapping failed");
        let result: Vec<f32> = bytemuck::cast_slice(&data)[..len].to_vec();
        drop(data);
        buffer.unmap();
        result
    }

    /// Non-blocking async readback - wasm32 only. WebGPU buffer mapping
    /// is unavoidably asynchronous in a browser; there is no blocking
    /// wait available there at all (see this module's own top-of-file
    /// doc comment and RFC-001's post-mortem on the removed attempt,
    /// which got this exact point wrong). Deliberately does not call
    /// `device.poll()` at all - the browser's own event loop drives GPU
    /// command processing, and polling isn't available/needed on the
    /// WebGPU backend the way it is natively.
    #[cfg(target_arch = "wasm32")]
    pub async fn read_buffer_async(&self, buffer: &wgpu::Buffer, len: usize) -> Vec<f32> {
        let slice = buffer.slice(..);
        let state = std::rc::Rc::new(std::cell::RefCell::new(MapReadyState { result: None, waker: None }));
        let callback_state = state.clone();

        slice.map_async(wgpu::MapMode::Read, move |result| {
            let mut state = callback_state.borrow_mut();
            state.result = Some(result);
            if let Some(waker) = state.waker.take() {
                waker.wake();
            }
        });

        MapReadyFuture { state: state.clone() }
            .await
            .expect("gpu buffer mapping failed");

        let data = slice.get_mapped_range().expect("gpu buffer mapping failed");
        let result: Vec<f32> = bytemuck::cast_slice(&data)[..len].to_vec();
        drop(data);
        buffer.unmap();
        result
    }
}

/// A hand-rolled `Future` that resolves once `map_async`'s callback
/// fires, without pulling in an extra async-utility crate for a single
/// oneshot signal. Single-threaded (wasm32 has no threads to race with),
/// so a plain `Rc<RefCell<...>>` is sufficient - no atomics needed.
#[cfg(target_arch = "wasm32")]
struct MapReadyState {
    result: Option<Result<(), wgpu::BufferAsyncError>>,
    waker: Option<std::task::Waker>,
}

#[cfg(target_arch = "wasm32")]
struct MapReadyFuture {
    state: std::rc::Rc<std::cell::RefCell<MapReadyState>>,
}

#[cfg(target_arch = "wasm32")]
impl std::future::Future for MapReadyFuture {
    type Output = Result<(), wgpu::BufferAsyncError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut state = self.state.borrow_mut();
        if let Some(result) = state.result.take() {
            std::task::Poll::Ready(result)
        } else {
            state.waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOUBLE_SHADER: &str = r#"
        @group(0) @binding(0) var<storage, read> input: array<f32>;
        @group(0) @binding(1) var<storage, read_write> output: array<f32>;

        @compute @workgroup_size(64)
        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
            if (id.x < arrayLength(&input)) {
                output[id.x] = input[id.x] * 2.0;
            }
        }
    "#;

    #[test]
    fn gpu_state_new_resolves_to_a_result_without_panicking() {
        // Whether or not this machine actually has a usable GPU adapter,
        // GpuState::new() must resolve to a Result rather than
        // panicking - Context.gpu/App.gpu's whole "None is a normal,
        // expected state" design depends on init failure being an
        // ordinary Err, never a crash.
        match pollster::block_on(GpuState::new()) {
            Ok(_) => {}
            Err(message) => assert!(!message.is_empty()),
        }
    }

    #[test]
    fn a_trivial_pipeline_round_trips_through_upload_dispatch_copy_and_readback() {
        let Ok(gpu) = pollster::block_on(GpuState::new()) else {
            eprintln!("skipping: no GPU adapter available in this environment");
            return;
        };

        let shader = gpu.create_shader(DOUBLE_SHADER);

        let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu module test bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline = gpu.create_compute_pipeline("gpu module test pipeline", &shader, "main", &[&bind_group_layout]);

        let input_data: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
        let byte_len = (input_data.len() * std::mem::size_of::<f32>()) as u64;

        let input_buffer = gpu.upload(
            "gpu module test input",
            &input_data,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let output_buffer = gpu.create_buffer(
            "gpu module test output",
            byte_len,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let readback_buffer = gpu.create_buffer(
            "gpu module test readback",
            byte_len,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );

        let bind_group = gpu.create_bind_group(
            "gpu module test bind group",
            &bind_group_layout,
            &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buffer.as_entire_binding() },
            ],
        );

        gpu.dispatch("gpu module test dispatch", &pipeline, &bind_group, (1, 1, 1));
        gpu.copy_buffer_to_buffer(&output_buffer, &readback_buffer, byte_len);

        let result = gpu.read_buffer_blocking(&readback_buffer, input_data.len());

        for (index, (input, output)) in input_data.iter().zip(result.iter()).enumerate() {
            assert!(
                (input * 2.0 - output).abs() < 1e-4,
                "index {}: expected {}, got {}",
                index,
                input * 2.0,
                output
            );
        }
    }
}
