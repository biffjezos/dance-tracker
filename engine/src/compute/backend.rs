pub trait ComputeBackend: Send + Sync {
    fn blur(
        &self,
        pixels: &[f32],
        width: u32,
        height: u32,
        radius: u32,
    ) -> Vec<f32>;
}