/** by biffjezos

    We cannot add new features and operations if we do not separate concerns.

    1. Position and movements should be separate:

    struct Position {
        x: f64,
        y: f64,
    }
    
    Use this struct for all nodes that need a position. Replace all x,y values in the other
    operations. 

    2.
    The rings and all other generated nodes should be static.
    The hard coded movement is an operation. BUT NOT BY DEFAULT
    only if a movement operation is added to a node (can be of many (visual) types ).

    It's up-to-you, but create an animation operator that does the same moves
    used for the Rings now. Give it an explicit name like GentleMove. 

    You could also implement a ```2dTransform``` operation that moves nodes in x and y.
    It might be a good design decision to have the animation operation use 2dTransform
    as long as the animation is active. (Play/Stop controls are already implemented).

    Please elaborate on the actual abstraction you are going to use.
    
    3. About ring(s), RINGS and Constellation
    
    The RINGS generator operation should

    - only create one RINGS node
    - which can have one or many rings (saturn-like rings)
    - each RINGS node can have one color (all rings of a RINGS node are the same color,
    as it is now)
    
    If a user wants more than one saturn RINGS, new RINGS nodes must be added.

    3.a) Separate Constellation from RINGS.
    
    - In the UI add a new MENU: 'ANIMATE' under which the user can find "CONSTELLATION"
    - Create a new Constellation operation in ```operations::animations::constellation.rs```
    - Constellation keeps two or more nodes at a set distance, use the same maths behind it
    to support many nodes at the same distance (in-between).
    (2 at a line, 3 in a triangle, 4 in a square, ..., 8 in an octagon)

    Example:
    The ANIMATE/CONSTELLATION sub menu:
    ADD - [NODE SELECTOR] + | - [DISTANCE] + | - [ NODES IN CONSTELLATION ] + | [future remove button]
    
    - ADD
    - - MASK 1
    - - MASK 2
    - - RINGS 1

    Added nodes appear in NODES IN CONSTELLATION.

    Menu: NODES/CONSTELLATION 1

    Renders MASK 1, MASK 2, RINGS 1 in a Constellation around a common CENTER. It's static and not
    animated at the moment. CONSTELLATION 1 appears under NODES:
    
    Menu: ANIMATE/[GENTLE MOVE] ( the animation currently hard coded into RINGS)
    ADD TO - [NODE SELECTOR] + | - [SPEED] + (and the settings currently used in RINGS for the animation)

    Menu: NODES/CONSTELLATION 1 selected

    Shows MASK 1, MASK 2, RINGS 1 in the triangle-constellation at set distance, speed and other properties
    
    4. 

    For the colours (completely gone from the UI):
    
    I added a ```struct Colour``` to ```compositor.rs``` and a stub for
    ``òperations::generators::SolidColor``` that struct
    instead ```pub colours: Vec<String>``` 

    I was thinking that we would use these to:

    - generate a solid plane (menu GENERATE Solid -> ``òperations::generators::SolidColor```)
    - to give nodes a color (text, masks, rings, solid plane, ghosts)
    - to change the color of the nodes.

    5.
    
    Go through all visualizing nodes and make them respect the new operations, if applicable.

    which means:

    - they need a colour to show up on the canvas,
    - they have a center position and they have
    - the have no movement operations attached by default

    From now on ALWAYS THINK OF separation of concerns. If a struct could potentially be re-used
    DO NOT PUT IT INTO THE LOWEST MODULE IN THE HIERARCHY. PUT IT INTO A HIGHER LEVEL AND USE IT.

    Check all operations. Does a module contain code that could be an operation? If yes, create
    an operation for it, or use an existing operation (all visual nodes may have a center and are
    positioned at the center). In the future the center may be movable in x,y or animated.

**/


/*
Draws via the browser's own Canvas2D (arcs/strokes) through web-sys on
a private, detached scratch canvas, then reads the result back into a
Frame - reimplementing anti-aliased circle stroking by hand in pure
Rust pixel math would be a lot of extra scope for a worse-looking
result than just using the same drawing primitives the old JS version
already relied on. wasm32-only for that reason, same as dom.rs.
*/
#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::f64::consts::PI;


use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::compositor::{
    Center, Color, Context, Input, Operation, OperationCategory, OperationError, OperationMetadata, OutputKind,
    ParameterDescriptor, ParameterKind, Value,
};
use crate::operations::Frame;

pub struct Rings {
    center: Center,
    ring_count: u32,
    spacing: f64,
    radius: f64,
    width: f64,
    colour: Color,
}

impl Rings {
    pub fn new(
        centre: Center,
        ring_count: u32,
        spacing: f64,
        radius: f64,
        stroke_width: f64,
        colour: Color,
    ) -> Result<Rings, JsValue> {
        let document = web_sys::window()
            .expect("window should exist")
            .document()
            .expect("document should exist");

        let canvas: HtmlCanvasElement = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;

        let ctx = canvas
            .get_context("2d")?
            .expect("canvas should have a 2d context")
            .dyn_into::<CanvasRenderingContext2d>()?;

        Ok(Rings {
            centre,
            ring_count,
            spacing,
            radius,
            width: stroke_width,
            colour,
            canvas,
            ctx,
        })
    }
}

impl Operation for Rings {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn metadata(&self) -> OperationMetadata {
        OperationMetadata {
            display_name: "Rings",
            category: OperationCategory::Generator,
            input_count: 0,
            outputs: vec![OutputKind::Frame],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "ring_count",
                kind: ParameterKind::Number,
            },
            ParameterDescriptor {
                name: "spacing",
                kind: ParameterKind::Number,
            },
            ParameterDescriptor {
                name: "radius",
                kind: ParameterKind::Number,
            },
            ParameterDescriptor {
                name: "stroke_width",
                kind: ParameterKind::Number,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "ring_count" => Some(Value::Number(self.ring_count as f64)),
            "spacing" => Some(Value::Number(self.spacing)),
            "radius" => Some(Value::Number(self.radius)),
            "stroke_width" => Some(Value::Number(self.width)),
            _ => None,
        }
    }

    fn set_parameter(
        &mut self,
        name: &str,
        value: Value,
    ) -> Result<(), OperationError> {
        match (name, value) {
            ("ring_count", Value::Number(v)) => {
                self.ring_count = v.max(0.0) as u32;
                Ok(())
            }

            ("spacing", Value::Number(v)) => {
                self.spacing = v;
                Ok(())
            }

            ("radius", Value::Number(v)) => {
                self.radius = v;
                Ok(())
            }

            ("stroke_width", Value::Number(v)) => {
                self.width = v;
                Ok(())
            }

            (
                "ring_count" |
                "spacing" |
                "radius" |
                "stroke_width",
                _
            ) => {
                Err(OperationError::WrongValueType)
            }

            _ => Err(OperationError::UnknownParameter(name.to_string())),
        }
    }

    fn execute(
        &self,
        ctx: &Context,
        _inputs: &[(Input, Value)],
    ) -> Result<Vec<Value>, OperationError> {
        self.canvas.set_width(ctx.meta.width);
        self.canvas.set_height(ctx.meta.height);

        let width = self.canvas.width();
        let height = self.canvas.height();

        self.ctx.clear_rect(
            0.0,
            0.0,
            width as f64,
            height as f64,
        );

        self.ctx.save();

        self.ctx.set_global_alpha(0.8);
        self.ctx.set_stroke_style_str(&self.colour.to_css());
        self.ctx.set_line_width(self.width);

        for n in 0..self.ring_count {
            let radius = self.radius + n as f64 * self.spacing;

            self.ctx.begin_path();

            self.ctx
                .arc(
                    self.center.x,
                    self.center.y,
                    radius,
                    0.0,
                    PI * 2.0,
                )
                .map_err(|_| OperationError::WrongValueType)?;

            self.ctx.stroke();
        }

        self.ctx.restore();

        let image_data = self
            .ctx
            .get_image_data(
                0.0,
                0.0,
                width as f64,
                height as f64,
            )
            .map_err(|_| OperationError::WrongValueType)?;

        let frame = Frame {
            pixels: image_data.data().0,
            width,
            height,
            timestamp: 0.0,
        };

        Ok(vec![
            Value::Frame(std::sync::Arc::new(frame))
        ])
    }
}
