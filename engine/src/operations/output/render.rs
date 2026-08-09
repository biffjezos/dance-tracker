// src/operations/output/render.rs
use std::any::Any;

use crate::compositor::{
    Context,
    Operation,
    OperationDescriptor,
    OperationError,
    Input,
    Value,
    metadata::{
        OperationCategory,
        OperationMetadata,
        OutputKind,
    },
};

/// v1 of SPEC-OUTPUT-RENDER-V1: a reachable trigger for the existing
/// (previously unreachable - nothing in `ui/` dispatched its event)
/// `Recorder`/`toggleRecord` mechanism, not a new recording pipeline.
/// Zero inputs, zero parameters, by design: RENDER records whatever the
/// OUTPUT/master canvas is currently showing via `canvas.captureStream()`
/// (`ui/scripts/engine/recorder.js`), the same way a camera pointed at a
/// screen would - it is not graph-aware and has no wire to "know what to
/// record". `execute()` only exists to satisfy the `Operation` trait so
/// RENDER can participate in the graph/menu system like every other
/// node; its real behavior (starting/stopping the browser recorder,
/// downloading the result) is a JS-side side effect triggered by
/// clicking EXECUTE in its EDIT screen
/// (`ui/scripts/engine/nodeEditContexts.js`), not anything the graph's
/// compute model produces.
///
/// Overrides `supports_edit()` to `true`: the trait's default
/// (`!parameters().is_empty() || input_count() > 0`) is a heuristic for
/// "has something to edit," which doesn't hold here - RENDER has
/// nothing to *edit* but still needs its EDIT screen reachable purely
/// for the EXECUTE button (`menu.js`'s `renderEditButton` gates the
/// EDIT button on this exact flag). FILE_NAME/FORMAT/RESOLUTION/FPS/
/// COMPRESSOR/FROM-TO are explicitly out of scope for this round - see
/// SPEC-OUTPUT-RENDER-V1's "Out of scope".
pub struct Render;

impl Render {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Render {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Render {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "render",
            menu: "OUTPUT",
            label: "RENDER",
            action: None,
            ui_action: None,
            create_node: Some("render"),
            submenu: None,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Render",
            category: OperationCategory::Output,
            inputs: vec![],
            outputs: vec![OutputKind::Boolean],
        }
    }

    fn supports_edit(&self) -> bool {
        true
    }

    fn execute(&self, _ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        Ok(vec![Value::Boolean(true)])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Render::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::graph::Graph;
    use crate::compositor::executors::{Execute, RenderExecutor};

    fn context() -> Context {
        Context {
            meta: crate::compositor::Meta { width: 4, height: 4, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn descriptor_matches_spec_output_render_v1() {
        let render = Render::new();
        let descriptor = render.descriptor();
        assert_eq!(descriptor.id, "render");
        assert_eq!(descriptor.menu, "OUTPUT");
        assert_eq!(descriptor.label, "RENDER");
        assert_eq!(descriptor.create_node, Some("render"));
        assert_eq!(descriptor.submenu, None);
    }

    #[test]
    fn metadata_declares_zero_inputs_under_the_output_category() {
        let render = Render::new();
        let metadata = render.metadata();
        assert_eq!(metadata.category, OperationCategory::Output);
        assert!(metadata.inputs.is_empty(), "v1 is not graph-aware - it captures the OUTPUT canvas directly");
    }

    #[test]
    fn parameters_are_empty_for_v1() {
        assert!(Render::new().parameters().is_empty(), "FILE_NAME/FORMAT/RESOLUTION/FPS/COMPRESSOR/FROM-TO are explicitly deferred");
    }

    #[test]
    fn supports_edit_is_true_despite_zero_inputs_and_parameters() {
        // The default heuristic (has params or inputs) would otherwise
        // hide RENDER's EDIT button entirely, making its EXECUTE button
        // unreachable - see this struct's own doc comment.
        assert!(Render::new().supports_edit(), "EDIT must stay reachable purely for the EXECUTE button");
    }

    #[test]
    fn execute_returns_a_harmless_stub_value() {
        let render = Render::new();
        let values = render.execute(&context(), &[]).unwrap();
        assert_eq!(values.len(), 1);
        assert!(matches!(values[0], Value::Boolean(true)));
    }

    #[test]
    fn render_in_graph_is_valid() {
        let mut graph = Graph::new(4, 4);
        let node_id = graph.add_node(Box::new(Render::new()));
        graph.validate().expect("unwired render is valid");
        RenderExecutor::new()
            .execute(&graph, node_id, &context())
            .expect("unwired render executes");
    }
}
