// src/operations/compose/patch.rs
use std::any::Any;
use std::sync::Arc;

use crate::compositor::{
    Context,
    Operation,
    OperationDescriptor,
    OperationError,
    Input,
    input::find_input,
    Value,
    metadata::{InputDescriptor, OperationCategory, OperationMetadata, OutputKind, ParameterDescriptor},
};

use crate::graphics::FloatImage;

/// Links an animation source's outputs to another node's own properties -
/// "nodes manipulate their inputs" (CLAUDE.md's UI direction), just with
/// SOURCE (the thing being manipulated) and REFERENCE (the animation
/// driving it) as PATCH's own two wired inputs, and the actual mapping
/// authored on PATCH's own edit screen. Two different things can happen
/// per mapped property, decided per-tick by `Graph::apply_patch_nodes`
/// (compositor/graph/drive.rs), not by this struct:
///
/// - SOURCE has a real parameter by that name (or a Color parameter's
///   decomposed channel) - it gets injected directly into SOURCE's own
///   `set_parameter()`, before SOURCE renders. PATCH's own output is
///   then just a plain passthrough of SOURCE's (now-animated) pixels.
/// - SOURCE has no such parameter (a plain pixel source with none at
///   all) - the mapped value lands in PATCH's own R/G/B/A state instead,
///   and PATCH's `execute()` (below) substitutes that channel across
///   SOURCE's rendered pixels itself.
///
/// R/G/B/A are intentionally not listed in `parameters()` - they're
/// written only by the pre-pass via `set_parameter()`, never a
/// user-facing control (the wiring/mapping UI is PATCH's actual
/// interface). Still routed through the same `set_parameter()` hook as
/// every other operation's state, per ANIMATION_CONVENTIONS.md - just
/// not advertised.
pub struct Patch {
    r_value: Option<f32>,
    g_value: Option<f32>,
    b_value: Option<f32>,
    a_value: Option<f32>,
}

impl Patch {
    pub fn new() -> Self {
        Self {
            r_value: None,
            g_value: None,
            b_value: None,
            a_value: None,
        }
    }
}

impl Default for Patch {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Patch {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "patch",
            menu: "COMPOSE",
            label: "PATCH",
            action: None,
            ui_action: None,
            create_node: Some("patch"),
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
            display_name: "Patch",
            category: OperationCategory::Composite,
            inputs: vec![
                // SOURCE is "the node being manipulated" (see this
                // operation's own doc comment) - it can be anything, so no
                // restriction.
                InputDescriptor { kind: Input::Source, accepts: &[] },
                InputDescriptor { kind: Input::Reference, accepts: &[OutputKind::Number] },
            ],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        Vec::new()
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "R" => self.r_value.map(|v| Value::Number(v as f64)),
            "G" => self.g_value.map(|v| Value::Number(v as f64)),
            "B" => self.b_value.map(|v| Value::Number(v as f64)),
            "A" => self.a_value.map(|v| Value::Number(v as f64)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("R", Value::Number(v)) => { self.r_value = Some(v as f32); Ok(()) }
            ("G", Value::Number(v)) => { self.g_value = Some(v as f32); Ok(()) }
            ("B", Value::Number(v)) => { self.b_value = Some(v as f32); Ok(()) }
            ("A", Value::Number(v)) => { self.a_value = Some(v as f32); Ok(()) }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    // R/G/B/A live outside parameters(), so RenderExecutor's cache
    // fingerprint (which only ever iterates parameters()) can never see
    // them change - without staying live, a pre-pass-driven channel
    // update would be silently served from a stale cached render.
    fn is_live(&self) -> bool {
        true
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(source) = find_input(inputs, Input::Source) else {
            return Err(OperationError::MissingInput("PATCH requires SOURCE".into()));
        };

        let source_image = FloatImage::from_value(source, ctx)?;
        let mut pixels = source_image.pixels.clone();

        for pixel in pixels.chunks_exact_mut(4) {
            if let Some(v) = self.r_value { pixel[0] = v; }
            if let Some(v) = self.g_value { pixel[1] = v; }
            if let Some(v) = self.b_value { pixel[2] = v; }
            if let Some(v) = self.a_value { pixel[3] = v; }
        }

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels,
            width: source_image.width,
            height: source_image.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Patch::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(width: u32, height: u32) -> Context {
        Context {
            meta: crate::compositor::Meta { width, height, ..Default::default() },
            ..Default::default()
        }
    }

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<crate::graphics::U8Image> {
        Arc::new(crate::graphics::U8Image { pixels, width, height, format: crate::graphics::ImageFormat::Rgba8 })
    }

    #[test]
    fn with_no_channels_set_source_passes_through_unchanged() {
        let patch = Patch::new();
        let source = Value::Image(image(vec![10, 20, 30, 255], 1, 1));

        let values = patch.execute(&context(1, 1), &[(Input::Source, source)]).unwrap();

        match &values[0] {
            Value::FloatImage(out) => {
                assert!((out.pixels[0] - 10.0 / 255.0).abs() < 1e-6);
                assert!((out.pixels[3] - 1.0).abs() < 1e-6);
            }
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn a_set_channel_overrides_every_pixel_uniformly() {
        let mut patch = Patch::new();
        patch.set_parameter("R", Value::Number(0.9)).unwrap();
        patch.set_parameter("A", Value::Number(0.2)).unwrap();

        let source = Value::Image(image(vec![10, 20, 30, 255, 40, 50, 60, 255], 2, 1));
        let values = patch.execute(&context(2, 1), &[(Input::Source, source)]).unwrap();

        match &values[0] {
            Value::FloatImage(out) => {
                // Both pixels get R and A overridden; G/B pass through.
                for pixel in out.pixels.chunks_exact(4) {
                    assert!((pixel[0] - 0.9).abs() < 1e-6);
                    assert!((pixel[3] - 0.2).abs() < 1e-6);
                }
                assert!((out.pixels[1] - 20.0 / 255.0).abs() < 1e-6, "G must pass through unchanged");
            }
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn execute_errors_without_a_wired_source() {
        let patch = Patch::new();
        let err = patch.execute(&context(1, 1), &[]).unwrap_err();
        assert!(matches!(err, OperationError::MissingInput(_)));
    }

    #[test]
    fn is_live_returns_true() {
        assert!(Patch::new().is_live(), "R/G/B/A live outside parameters() and must never be served from a stale cache");
    }

    #[test]
    fn set_parameter_rejects_an_unknown_channel_name() {
        let mut patch = Patch::new();
        let err = patch.set_parameter("Q", Value::Number(0.5)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterType(_)));
    }

    #[test]
    fn source_input_accepts_is_unrestricted() {
        let metadata = Patch::new().metadata();
        let source = metadata.inputs.iter().find(|d| d.kind == Input::Source).unwrap();
        assert!(source.accepts.is_empty(), "SOURCE can be anything - the empty-accepts escape hatch");
    }

    #[test]
    fn reference_input_accepts_exactly_number() {
        let metadata = Patch::new().metadata();
        let reference = metadata.inputs.iter().find(|d| d.kind == Input::Reference).unwrap();
        assert_eq!(reference.accepts, &[OutputKind::Number]);
    }
}
