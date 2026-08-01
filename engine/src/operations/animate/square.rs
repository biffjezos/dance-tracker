// src/operations/animate/square.rs
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
        ParameterDescriptor,
        ParameterKind,
    },
};

/// A hard on/off wave - useful for strobe-like effects a smooth Sine
/// can't produce. DUTY_CYCLE is the fraction of each period spent "high"
/// (0.5 = a symmetric square wave; smaller values produce short pulses).
pub struct Square {
    pub frequency: f64,
    pub duty_cycle: f64,
    pub amplitude: f64,
    pub offset: f64,
}

impl Square {
    pub fn new() -> Self {
        Self {
            frequency: 1.0,
            duty_cycle: 0.5,
            amplitude: 1.0,
            offset: 0.0,
        }
    }

    pub fn sample(&self, time: f64) -> f64 {
        let phase_fraction = (self.frequency * time).rem_euclid(1.0);
        let high = phase_fraction < self.duty_cycle;
        self.offset + if high { self.amplitude } else { -self.amplitude }
    }
}

impl Default for Square {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Square {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "square",
            menu: "ANIMATE",
            label: "SQUARE",
            action: None,
            ui_action: None,
            create_node: Some("square"),
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
            display_name: "Square",
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
                name: "DUTY_CYCLE",
                kind: ParameterKind::Number { step: 0.05, min: Some(0.0), max: Some(1.0) },
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
            "DUTY_CYCLE" => Some(Value::Number(self.duty_cycle)),
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
            ("DUTY_CYCLE", Value::Number(v)) => {
                if !(0.0..=1.0).contains(&v) {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.duty_cycle = v;
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
        constructor: || Box::new(Square::new())
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
    fn starts_high_at_time_zero() {
        let square = Square::new();
        assert_eq!(square.sample(0.0), 1.0);
    }

    #[test]
    fn goes_low_past_the_duty_cycle_fraction_of_the_period() {
        let square = Square { frequency: 1.0, duty_cycle: 0.25, amplitude: 1.0, offset: 0.0 };
        assert_eq!(square.sample(0.5), -1.0, "half a period in, well past a 0.25 duty cycle");
    }

    #[test]
    fn offset_shifts_both_the_high_and_low_states() {
        let square = Square { frequency: 1.0, duty_cycle: 0.5, amplitude: 1.0, offset: 5.0 };
        assert_eq!(square.sample(0.0), 6.0);
    }

    #[test]
    fn is_live_returns_true() {
        assert!(Square::new().is_live(), "must stay live or the animation freezes after tick one");
    }

    #[test]
    fn set_parameter_rejects_a_duty_cycle_out_of_range() {
        let mut square = Square::new();
        let err = square.set_parameter("DUTY_CYCLE", Value::Number(1.5)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn square_in_graph_is_valid() {
        let mut graph = Graph::new(4, 4);
        let node_id = graph.add_node(Box::new(Square::new()));
        graph.validate().expect("unwired square is valid");
        RenderExecutor::new()
            .execute(&graph, node_id, &context(0.0))
            .expect("unwired square renders");
    }
}
