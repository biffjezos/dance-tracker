// src/profiling.rs

#[derive(Debug)]
pub struct ProfileEntry {
    pub name: &'static str,
    pub duration_ms: f64,
}

#[derive(Debug)]
pub struct Profile {
    pub entries: Vec<ProfileEntry>,
    pub total_ms: f64,
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for entry in &self.entries {
            writeln!(f, "{}: {:.1}ms", entry.name, entry.duration_ms)?;
        }
        write!(f, "Total: {:.1}ms", self.total_ms)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn measure_ms<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let start = std::time::Instant::now();
    let result = f();
    (result, start.elapsed().as_secs_f64() * 1000.0)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn measure_ms<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let perf = web_sys::window().and_then(|w| w.performance());
    let start = perf.as_ref().map(|p| p.now()).unwrap_or(0.0);
    let result = f();
    let elapsed = perf.as_ref().map(|p| p.now() - start).unwrap_or(0.0);
    (result, elapsed)
}
