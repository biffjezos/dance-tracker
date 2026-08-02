// src/compositor/operations.rs

use std::any::Any;
use std::sync::Arc;

use crate::operations::sources::PixelSource;
use crate::compositor::{
    bbox::Rect,
    Context,
    OperationDescriptor,
    OperationError,
    input::Input,
    metadata::{ OperationMetadata, ParameterDescriptor },
    Value
};
pub trait Operation: Any {
    fn descriptor(&self) -> OperationDescriptor;
    
    fn execute(
        &self,
        ctx: &Context,
        inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError>;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn metadata(&self) -> OperationMetadata;

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        Vec::new()
    }

    /// Human-readable label for each entry in `metadata().outputs`, in
    /// the same order - e.g. Lissajous's two Number outputs are "X" and
    /// "Y", not indistinguishable by position. Empty by default (most
    /// operations have exactly one output and don't need one labelled);
    /// only operations meant to be picked as a PATCH node's animation
    /// source need to override this. Purely cosmetic - nothing in the
    /// engine depends on these strings, they only reach the UI.
    fn output_names(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn get_parameter(&self, _name: &str) -> Option<Value> {
        None
    }

    fn set_parameter(&mut self, name: &str, _value: Value) -> Result<(), OperationError> {
        Err(OperationError::UnknownParameter(name.to_string()))
    }

    /// Returns true if this operation supports editing - it has parameters
    /// that can be modified, and/or inputs that can be rewired.
    fn supports_edit(&self) -> bool {
        !self.parameters().is_empty() || self.metadata().input_count() > 0
    }

    /// Attach a live pixel source to this operation.
    ///
    /// Source operations that pull their pixels from something the host owns
    /// (a camera stream, a decoded video element) accept one here, so the
    /// host side never needs to know which concrete operation it is talking to.
    fn set_pixel_source(
        &mut self,
        _source: Arc<dyn PixelSource>,
    ) -> Result<(), OperationError> {
        Err(OperationError::NotImplemented(
            format!(
                "{} does not read from a pixel source",
                self.metadata().display_name
            )
        ))
    }

    /// Whether this operation's output can change from tick to tick with
    /// neither its own parameters nor its resolved inputs changing - true
    /// for anything pulling frames out of an external, host-owned source
    /// (a playing camera stream, a decoded video element). A cross-tick
    /// cache keyed only on parameters + inputs (see RenderExecutor) has no
    /// way to see that kind of change, so a live operation must always be
    /// re-executed rather than served from that cache.
    fn is_live(&self) -> bool {
        false
    }

    /// The region of this operation's own just-computed `output` that
    /// actually matters - see BBOX_CONVENTIONS.md. The default (full
    /// frame) is exactly today's implicit behavior and is always safe;
    /// overriding it to something tighter is a report-only optimization
    /// hint for downstream nodes, never a change to this operation's own
    /// output pixels. `output` is unused by every operation in this
    /// round (their boxes are derivable from parameters/input boxes
    /// alone) - it exists so a future content-derived box doesn't need a
    /// second trait-signature change.
    fn output_bbox(&self, ctx: &Context, _input_bboxes: &[(Input, Rect)], _output: &Value) -> Rect {
        Rect::full(ctx.meta.width, ctx.meta.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::metadata::{OperationCategory, OperationMetadata};
    use crate::compositor::operation_descriptor::OperationDescriptor;

    /// An operation that overrides nothing bbox-related - exactly what
    /// every operation in the tree looked like before this round.
    struct PlainOperation;

    impl Operation for PlainOperation {
        fn descriptor(&self) -> OperationDescriptor {
            OperationDescriptor {
                id: "plain",
                menu: "TEST",
                label: "PLAIN",
                action: None,
                ui_action: None,
                create_node: None,
                submenu: None,
            }
        }

        fn as_any(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }

        fn metadata(&self) -> OperationMetadata {
            OperationMetadata {
                display_name: "Plain",
                category: OperationCategory::Source,
                inputs: vec![],
                outputs: vec![],
            }
        }

        fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
            Ok(vec![])
        }
    }

    #[test]
    fn the_default_output_bbox_is_exactly_full_frame() {
        let ctx = Context {
            meta: crate::compositor::Meta { width: 640, height: 360, ..Default::default() },
            ..Default::default()
        };

        let bbox = PlainOperation.output_bbox(&ctx, &[], &Value::Number(0.0));

        assert_eq!(bbox, Rect::full(640, 360));
    }
}
