#![cfg(target_arch = "wasm32")]

use web_sys::HtmlVideoElement;

use crate::compositor::{Context, Input, Operation, OperationError, Value};

/*
Covers MINUTE +/SECOND +/FRAME + alike - just different seconds
values (60.0, 1.0, 1.0/30.0) wired at construction time.
*/
pub struct Forward {
    pub video: HtmlVideoElement,
    pub seconds: f64,
}

impl Operation for Forward {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn execute(
        &self,
        _ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        let target = self.video.current_time() + self.seconds;

        self.video.set_current_time(target);

        Ok(vec![])
    }
}
