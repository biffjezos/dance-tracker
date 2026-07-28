
use crate::compositor::{
    find_input, Center, Context, Input, Operation, OperationCategory, OperationError, OperationMetadata,
    OutputKind, Point2d, Value,
};
pub struct Lissajous {
    center: Center,
    amplitude_x: f64,
    amplitude_y: f64,
    frequency_x: f64,
    frequency_y: f64,
    phase: f64,
}

impl Operation for Lissajous {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Lissajous Movement",
            category: OperationCategory::Generator,
            input_count: 0,
            outputs: vec![OutputKind::Center],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "amplitude_x",
                kind: ParameterKind::Number,
            },
            ParameterDescriptor {
                name: "amplitude_y",
                kind: ParameterKind::Number,
            },
            ParameterDescriptor {
                name: "frequency_x",
                kind: ParameterKind::Number,
            },
            ParameterDescriptor {
                name: "frequency_y",
                kind: ParameterKind::Number,
            },
            ParameterDescriptor {
                name: "phase",
                kind: ParameterKind::Number,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "amplitude_x" => Some(Value::Number(self.amplitude_x)),
            "amplitude_y" => Some(Value::Number(self.amplitude_y)),
            "frequency_x" => Some(Value::Number(self.frequency_x)),
            "frequency_y" => Some(Value::Number(self.frequency_y)),
            "phase" => Some(Value::Number(self.phase)),
            _ => None,
        }
    }

    fn set_parameter(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<(), OperationError> {
        match (name, value) {
            ("amplitude_x", Value::Number(v)) => {
                self.amplitude_x = v;
                Ok(())
            }

            ("amplitude_y", Value::Number(v)) => {
                self.amplitude_y = v;
                Ok(())
            }

            ("frequency_x", Value::Number(v)) => {
                self.frequency_x = v;
                Ok(())
            }

            ("frequency_y", Value::Number(v)) => {
                self.frequency_y = v;
                Ok(())
            }

            ("phase", Value::Number(v)) => {
                self.phase = v;
                Ok(())
            }

            _ => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    fn execute(
        &self,
        ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {

        let time = ctx.meta.time;

        let x = self.center.point.x
            + (time * self.frequency_x + self.phase).sin()
            * self.amplitude_x;

        let y = self.center.point.y
            + (time * self.frequency_y + self.phase).cos()
            * self.amplitude_y;

        let result = Center {
            point: Point2D {
                x,
                y,
            },
        };

        Ok(vec![
            Value::Center(result)
        ])
    }
}