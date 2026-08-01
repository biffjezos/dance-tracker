// src/operations/generators/ring.rs
use std::any::Any;
use std::sync::Arc;

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

use crate::graphics::{Color, U8Image, ImageFormat};

/// Concentric rings, sized like Saturn's rings: RADIUS is the outer
/// edge of the whole set, SPACING is the gap between consecutive rings,
/// THICKNESS is the (uniform) stroke width of every ring. Static - no
/// time dependency, no `is_live()` - purely a function of its own
/// parameters, same as Checkerboard. Each ring gets its own colour via
/// RING_SELECTOR (bounded by the live COUNT) + RING_COLOR, in a
/// "COLOUR" parameter group - the same deep-menu mechanism
/// Checkerboard's A/B colours already use, just with an index selector
/// instead of two fixed named colours.
pub struct Ring {
    pub count: usize,
    pub radius: f64,
    pub spacing: f64,
    pub thickness: f64,
    selected_ring: usize, // 1-based, always in 1..=count
    colors: Vec<Color>,   // always exactly `count` long
}

impl Ring {
    pub fn new() -> Self {
        Self {
            count: 1,
            radius: 64.0,
            spacing: 16.0,
            thickness: 4.0,
            selected_ring: 1,
            colors: vec![Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }],
        }
    }

    /// Resize `colors` to exactly `new_count` entries, cloning the last
    /// ring's colour into any newly-added slots (a reasonable default
    /// fill, not left unexplained - see ANIMATION_IMPLEMENTATION_PLAN.md's
    /// RING section) and clamping `selected_ring` back into range if it
    /// no longer fits.
    fn set_count(&mut self, new_count: usize) {
        let new_count = new_count.max(1);

        if new_count > self.colors.len() {
            let fill = *self.colors.last().unwrap();
            self.colors.resize(new_count, fill);
        } else {
            self.colors.truncate(new_count);
        }

        self.count = new_count;
        self.selected_ring = self.selected_ring.min(self.count);
    }

    /// Ring `n`'s (1-based) own radius - ring 1 is outermost, at
    /// `RADIUS`; each subsequent ring sits `SPACING` further in.
    fn ring_radius(&self, ring_number: usize) -> f64 {
        self.radius - (ring_number as f64 - 1.0) * self.spacing
    }

    pub fn generate(&self, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let half_thickness = self.thickness.max(0.0) / 2.0;

        for y in 0..height {
            for x in 0..width {
                let dx = x as f64 + 0.5 - cx;
                let dy = y as f64 + 0.5 - cy;
                let dist = (dx * dx + dy * dy).sqrt();

                let index = ((y * width + x) * 4) as usize;

                for ring_number in 1..=self.count {
                    let ring_radius = self.ring_radius(ring_number);
                    if ring_radius < 0.0 {
                        continue;
                    }
                    if (dist - ring_radius).abs() <= half_thickness {
                        let rgba = self.colors[ring_number - 1].to_rgba_u8();
                        pixels[index..index + 4].copy_from_slice(&rgba);
                        break;
                    }
                }
            }
        }

        pixels
    }
}

impl Default for Ring {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Ring {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "ring",
            menu: "GENERATE",
            label: "RING",
            action: None,
            ui_action: None,
            create_node: Some("ring"),
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
            display_name: "Ring",
            category: OperationCategory::Generator,
            inputs: vec![],
            outputs: vec![OutputKind::Image],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "COUNT",
                kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "RADIUS",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "SPACING",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "THICKNESS",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "RING_SELECTOR",
                kind: ParameterKind::Number { step: 1.0, min: Some(1.0), max: Some(self.count as f64) },
                group: Some("COLOUR"),
            },
            ParameterDescriptor {
                name: "RING_COLOR",
                kind: ParameterKind::Color,
                group: Some("COLOUR"),
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "COUNT" => Some(Value::Number(self.count as f64)),
            "RADIUS" => Some(Value::Number(self.radius)),
            "SPACING" => Some(Value::Number(self.spacing)),
            "THICKNESS" => Some(Value::Number(self.thickness)),
            "RING_SELECTOR" => Some(Value::Number(self.selected_ring as f64)),
            "RING_COLOR" => Some(Value::Color(self.colors[self.selected_ring - 1])),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("COUNT", Value::Number(v)) => {
                if v < 1.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.set_count(v.round() as usize);
                Ok(())
            }
            ("RADIUS", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.radius = v;
                Ok(())
            }
            ("SPACING", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.spacing = v;
                Ok(())
            }
            ("THICKNESS", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.thickness = v;
                Ok(())
            }
            ("RING_SELECTOR", Value::Number(v)) => {
                let index = v.round() as i64;
                if index < 1 || index as usize > self.count {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.selected_ring = index as usize;
                Ok(())
            }
            ("RING_COLOR", Value::Color(color)) => {
                let index = self.selected_ring - 1;
                self.colors[index] = color;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, _inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        Ok(vec![
            Value::Image(Arc::new(U8Image {
                pixels: self.generate(ctx.meta.width, ctx.meta.height),
                width: ctx.meta.width,
                height: ctx.meta.height,
                format: ImageFormat::Rgba8,
            }))
        ])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Ring::new())
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

    #[test]
    fn a_single_ring_draws_a_band_at_its_radius() {
        let mut ring = Ring::new();
        ring.radius = 2.0;
        ring.thickness = 1.0;

        let pixels = ring.generate(8, 8);
        // Centre pixel is far from radius 2 - must stay transparent.
        let centre_index = ((4 * 8 + 4) * 4) as usize;
        assert_eq!(&pixels[centre_index..centre_index + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn count_bounds_the_ring_selector_parameter() {
        let mut ring = Ring::new();
        ring.set_parameter("COUNT", Value::Number(3.0)).unwrap();

        let selector = ring.parameters().into_iter().find(|p| p.name == "RING_SELECTOR").unwrap();
        assert_eq!(selector.kind.max(), Some(3.0), "the selector's own max must track live COUNT, never a fixed ceiling");
    }

    #[test]
    fn ring_selector_rejects_an_index_past_the_live_count() {
        let mut ring = Ring::new(); // COUNT defaults to 1
        let err = ring.set_parameter("RING_SELECTOR", Value::Number(2.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn each_ring_can_be_given_its_own_colour() {
        let mut ring = Ring::new();
        ring.set_parameter("COUNT", Value::Number(2.0)).unwrap();

        ring.set_parameter("RING_SELECTOR", Value::Number(1.0)).unwrap();
        ring.set_parameter("RING_COLOR", Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 })).unwrap();

        ring.set_parameter("RING_SELECTOR", Value::Number(2.0)).unwrap();
        ring.set_parameter("RING_COLOR", Value::Color(Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 })).unwrap();

        assert_eq!(ring.colors[0], Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        assert_eq!(ring.colors[1], Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 });
    }

    #[test]
    fn growing_count_fills_new_rings_with_the_last_rings_colour() {
        let mut ring = Ring::new();
        ring.set_parameter("RING_COLOR", Value::Color(Color { r: 0.2, g: 0.4, b: 0.6, a: 1.0 })).unwrap();
        ring.set_parameter("COUNT", Value::Number(3.0)).unwrap();

        assert_eq!(ring.colors.len(), 3);
        assert_eq!(ring.colors[1], Color { r: 0.2, g: 0.4, b: 0.6, a: 1.0 });
        assert_eq!(ring.colors[2], Color { r: 0.2, g: 0.4, b: 0.6, a: 1.0 });
    }

    #[test]
    fn shrinking_count_clamps_an_out_of_range_selection() {
        let mut ring = Ring::new();
        ring.set_parameter("COUNT", Value::Number(3.0)).unwrap();
        ring.set_parameter("RING_SELECTOR", Value::Number(3.0)).unwrap();

        ring.set_parameter("COUNT", Value::Number(1.0)).unwrap();
        assert_eq!(ring.selected_ring, 1, "selection must be clamped back into range, not left dangling");
    }

    #[test]
    fn set_parameter_rejects_a_negative_radius() {
        let mut ring = Ring::new();
        let err = ring.set_parameter("RADIUS", Value::Number(-1.0)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn ring_in_graph_is_valid() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, RenderExecutor};

        let mut graph = Graph::new(8, 8);
        let ring_id = graph.add_node(Box::new(Ring::new()));
        graph.validate().expect("unwired ring is valid");
        RenderExecutor::new()
            .execute(&graph, ring_id, &context(8, 8))
            .expect("unwired ring renders");
    }
}
