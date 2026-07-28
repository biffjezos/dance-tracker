/*
TODO/3.md #6: lightweight, optional per-operation timing.

Entirely opt-in - RenderExecutor::execute (the real per-frame path) is
untouched by this file. execute_profiled is a separate entry point a
caller reaches for only when it wants a breakdown, so profiling costs
nothing unless something actually calls it, and there's no branch to
skip in the render loop itself.

measure_ms needs a platform split because std::time::Instant panics at
runtime on wasm32-unknown-unknown (no OS clock) - the only place this
ever actually runs in production is the browser, so the wasm32 arm
using web_sys::Performance is the one that matters; the native arm
exists so cargo test can exercise the same code path.
*/

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
