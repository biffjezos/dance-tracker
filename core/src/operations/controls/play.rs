#![cfg(target_arch = "wasm32")]

use web_sys::HtmlVideoElement;

use crate::compositor::{Context, Operation, OperationError, Value};

pub struct Play {
    pub video: HtmlVideoElement,
}

impl Operation for Play {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn execute(
        &self,
        _ctx: &Context,
        _inputs: &[Value],
    ) -> Result<Vec<Value>, OperationError> {
        let _ = self.video.play().map_err(|_| OperationError::WrongValueType)?;

        Ok(vec![])
    }
}
