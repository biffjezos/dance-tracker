use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::compositor::{
    Context, Input, Operation, OperationCategory, OperationError, OperationMetadata, OutputKind,
    Value,
};
use crate::operations::Frame;

/*
A settable frame with no inputs of its own - backs "CAPTURE
BACKGROUND" for Difference matting (wire this in as its reference
input, then replace what it holds whenever the user re-captures)
without Difference itself needing any internal state or the Operation
trait needing an out-of-band "command" mechanism. Whoever constructs
this node keeps the Rc from handle() (taken before the CapturedFrame
is boxed into Box<dyn Operation> and erased) in its own side-table
keyed by NodeId, and writes into it directly on capture. Holds an Arc
so capturing is a refcount bump into place, not a pixel copy, and
execute() below hands the same Arc back out rather than cloning pixels
on every tick.
*/
pub struct CapturedFrame {
    frame: Rc<RefCell<Option<Arc<Frame>>>>,
}

impl CapturedFrame {
    pub fn new() -> Self {
        CapturedFrame {
            frame: Rc::new(RefCell::new(None)),
        }
    }

    pub fn handle(&self) -> Rc<RefCell<Option<Arc<Frame>>>> {
        self.frame.clone()
    }
}

impl Default for CapturedFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for CapturedFrame {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Captured Frame",
            category: OperationCategory::Reference,
            input_count: 0,
            outputs: vec![OutputKind::Frame],
        }
    }

    fn execute(
        &self,
        _ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        let frame = self
            .frame
            .borrow()
            .clone()
            .ok_or_else(|| OperationError::SourceNotFound("not captured yet".to_string()))?;

        Ok(vec![Value::Frame(frame)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_until_something_is_captured() {
        let node = CapturedFrame::new();
        let ctx = Context::default();

        let result = node.execute(&ctx, &[]);

        assert!(matches!(result, Err(OperationError::SourceNotFound(_))));
    }

    #[test]
    fn returns_whatever_was_set_through_the_handle() {
        let node = CapturedFrame::new();
        let handle = node.handle();

        *handle.borrow_mut() = Some(Arc::new(Frame {
            pixels: vec![9, 9, 9, 255],
            width: 1,
            height: 1,
            timestamp: 0.0,
        }));

        let ctx = Context::default();
        let mut outputs = node.execute(&ctx, &[]).expect("should succeed");

        let frame = match outputs.remove(0) {
            Value::Frame(frame) => frame,
            _ => panic!("should be a Frame"),
        };

        assert_eq!(frame.pixels, vec![9, 9, 9, 255]);
    }
}
