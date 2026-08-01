// src/operations/animate/lissajous.rs
use std::any::Any;
use std::f64::consts::TAU;

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
        ParameterDescriptor,
        ParameterKind,
    },
};

/// Two independent sine waves (X and Y), the classic Lissajous figure.
/// Purely a function of `ctx.meta.time` - no pixel input, no pixel
/// output. Unwired, this does nothing (per CLAUDE.md's "no default
/// anything"); it becomes useful once its X/Y outputs are wired into
/// another node's Number parameter (see ANIMATION_IMPLEMENTATION_PLAN.md
/// Phase C - not implemented yet, so today this only exists as a
/// standalone, independently testable operation).
pub struct Lissajous {
    pub freq_x: f64,
    pub freq_y: f64,
    pub phase_degrees: f64,
    pub amplitude: f64,
}

impl Lissajous {
    pub fn new() -> Self {
        Self {
            freq_x: 3.0,
            freq_y: 2.0,
            phase_degrees: 0.0,
            amplitude: 1.0,
        }
    }

    /// (x, y) at a given time - the pure math, kept separate from
    /// `execute()` so it's directly testable without a Context.
    pub fn sample(&self, time: f64) -> (f64, f64) {
        let phase = self.phase_degrees.to_radians();
        let x = self.amplitude * (TAU * self.freq_x * time + phase).sin();
        let y = self.amplitude * (TAU * self.freq_y * time).sin();
        (x, y)
    }
}

impl Default for Lissajous {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Lissajous {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "lissajous",
            menu: "ANIMATE",
            label: "LISSAJOUS",
            action: None,
            ui_action: None,
            create_node: Some("lissajous"),
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
            display_name: "Lissajous",
            category: OperationCategory::Animation,
            inputs: vec![],
            outputs: vec![OutputKind::Number, OutputKind::Number],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "FREQ_X",
                kind: ParameterKind::Number { step: 0.1, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "FREQ_Y",
                kind: ParameterKind::Number { step: 0.1, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "PHASE",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: Some(360.0) },
                group: None,
            },
            ParameterDescriptor {
                name: "AMPLITUDE",
                kind: ParameterKind::Number { step: 0.1, min: Some(0.0), max: None },
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "FREQ_X" => Some(Value::Number(self.freq_x)),
            "FREQ_Y" => Some(Value::Number(self.freq_y)),
            "PHASE" => Some(Value::Number(self.phase_degrees)),
            "AMPLITUDE" => Some(Value::Number(self.amplitude)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("FREQ_X", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.freq_x = v;
                Ok(())
            }
            ("FREQ_Y", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.freq_y = v;
                Ok(())
            }
            ("PHASE", Value::Number(v)) => {
                if !(0.0..=360.0).contains(&v) {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.phase_degrees = v;
                Ok(())
            }
            ("AMPLITUDE", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.amplitude = v;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    // Must stay live: nothing else about this operation's parameters or
    // (nonexistent) inputs changes tick to tick, so without this the
    // cross-tick render cache (RenderExecutor, keyed on parameter
    // fingerprint + resolved inputs only, never on ctx.meta.time) would
    // serve the first tick's output forever and the animation would
    // visibly freeze.
    fn is_live(&self) -> bool {
        true
    }

    fn execute(&self, ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let (x, y) = self.sample(ctx.meta.time);
        Ok(vec![Value::Number(x), Value::Number(y)])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Lissajous::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::graph::Graph;
    use crate::compositor::executors::{Execute, RenderExecutor};

    fn context(time: f64) -> Context {
        Context {
            meta: crate::compositor::Meta { time, width: 4, height: 4, ..Default::default() },
            ..Default::default()
        }
    }

    #[test]
    fn at_time_zero_with_zero_phase_both_axes_start_at_the_origin() {
        let lissajous = Lissajous::new();
        let (x, y) = lissajous.sample(0.0);
        assert!(x.abs() < 1e-9, "expected x ~= 0 at t=0, got {}", x);
        assert!(y.abs() < 1e-9, "expected y ~= 0 at t=0, got {}", y);
    }

    #[test]
    fn x_and_y_are_independent_when_frequencies_differ() {
        // t=0.25: freq_y*t = 0.5 cycles -> y = sin(pi) ~= 0;
        // freq_x*t = 0.75 cycles -> x = sin(1.5*pi) = -1. Clearly diverges.
        let lissajous = Lissajous { freq_x: 3.0, freq_y: 2.0, phase_degrees: 0.0, amplitude: 1.0 };
        let (x, y) = lissajous.sample(0.25);
        assert!((x - y).abs() > 0.5, "expected x and y to diverge, got x={} y={}", x, y);
    }

    #[test]
    fn amplitude_scales_the_output_range() {
        let unit = Lissajous { freq_x: 1.0, freq_y: 1.0, phase_degrees: 90.0, amplitude: 1.0 };
        let doubled = Lissajous { freq_x: 1.0, freq_y: 1.0, phase_degrees: 90.0, amplitude: 2.0 };
        let (x1, _) = unit.sample(0.0);
        let (x2, _) = doubled.sample(0.0);
        assert!((x2 - 2.0 * x1).abs() < 1e-9);
    }

    #[test]
    fn is_live_returns_true() {
        assert!(Lissajous::new().is_live(), "must stay live or the animation freezes after tick one");
    }

    #[test]
    fn set_parameter_rejects_a_negative_amplitude() {
        let mut lissajous = Lissajous::new();
        let err = lissajous.set_parameter("AMPLITUDE", Value::Number(-1.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn set_parameter_rejects_a_phase_out_of_range() {
        let mut lissajous = Lissajous::new();
        let err = lissajous.set_parameter("PHASE", Value::Number(400.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn lissajous_in_graph_is_valid() {
        let mut graph = Graph::new(4, 4);
        let node_id = graph.add_node(Box::new(Lissajous::new()));
        graph.validate().expect("unwired lissajous is valid");
        RenderExecutor::new()
            .execute(&graph, node_id, &context(0.0))
            .expect("unwired lissajous renders");
    }
}
