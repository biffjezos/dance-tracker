// src/operations/animate/sine.rs
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

/// A single sine wave - the simplest possible animation signal. Purely a
/// function of `ctx.meta.time`, same shape as Lissajous but one output
/// instead of two.
pub struct Sine {
    pub frequency: f64,
    pub phase_degrees: f64,
    pub amplitude: f64,
    pub offset: f64,
}

impl Sine {
    pub fn new() -> Self {
        Self {
            frequency: 1.0,
            phase_degrees: 0.0,
            amplitude: 1.0,
            offset: 0.0,
        }
    }

    pub fn sample(&self, time: f64) -> f64 {
        let phase = self.phase_degrees.to_radians();
        self.offset + self.amplitude * (TAU * self.frequency * time + phase).sin()
    }
}

impl Default for Sine {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Sine {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "sine",
            menu: "ANIMATE",
            label: "SINE",
            action: None,
            ui_action: None,
            create_node: Some("sine"),
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
            display_name: "Sine",
            category: OperationCategory::Animation,
            inputs: vec![],
            outputs: vec![OutputKind::Number],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "FREQUENCY",
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
            ParameterDescriptor {
                name: "OFFSET",
                kind: ParameterKind::Number { step: 0.1, min: None, max: None },
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "FREQUENCY" => Some(Value::Number(self.frequency)),
            "PHASE" => Some(Value::Number(self.phase_degrees)),
            "AMPLITUDE" => Some(Value::Number(self.amplitude)),
            "OFFSET" => Some(Value::Number(self.offset)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("FREQUENCY", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.frequency = v;
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
            ("OFFSET", Value::Number(v)) => {
                self.offset = v;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn is_live(&self) -> bool {
        true
    }

    fn execute(&self, ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        Ok(vec![Value::Number(self.sample(ctx.meta.time))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Sine::new())
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
    fn at_time_zero_with_zero_phase_and_offset_the_output_is_zero() {
        let sine = Sine::new();
        assert!(sine.sample(0.0).abs() < 1e-9);
    }

    #[test]
    fn offset_shifts_the_whole_wave() {
        let sine = Sine { frequency: 1.0, phase_degrees: 0.0, amplitude: 1.0, offset: 5.0 };
        assert!((sine.sample(0.0) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn amplitude_scales_the_output_range() {
        let unit = Sine { frequency: 1.0, phase_degrees: 90.0, amplitude: 1.0, offset: 0.0 };
        let doubled = Sine { frequency: 1.0, phase_degrees: 90.0, amplitude: 2.0, offset: 0.0 };
        assert!((doubled.sample(0.0) - 2.0 * unit.sample(0.0)).abs() < 1e-9);
    }

    #[test]
    fn is_live_returns_true() {
        assert!(Sine::new().is_live(), "must stay live or the animation freezes after tick one");
    }

    #[test]
    fn set_parameter_rejects_a_negative_frequency() {
        let mut sine = Sine::new();
        let err = sine.set_parameter("FREQUENCY", Value::Number(-1.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn sine_in_graph_is_valid() {
        let mut graph = Graph::new(4, 4);
        let node_id = graph.add_node(Box::new(Sine::new()));
        graph.validate().expect("unwired sine is valid");
        RenderExecutor::new()
            .execute(&graph, node_id, &context(0.0))
            .expect("unwired sine renders");
    }
}
