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

use crate::compositor::{Context, Operation, OperationError, Value};
use crate::operations::Frame;

struct Centre {
    x: f64,
    y: f64,
    phase: f64,
    speed: f64,
}

pub struct Rings {
    pub count: u32,
    pub rings_per_group: u32,
    pub spacing: f64,
    pub size: f64,
    pub width: f64,
    pub colours: Vec<String>,
    pub constellation: Option<f64>, // Some(distance) when enabled

    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    time: RefCell<f64>,
    hub_phase: f64,
    centres: RefCell<Vec<Centre>>,
}

impl Rings {
    pub fn new(
        width: u32,
        height: u32,
        count: u32,
        rings_per_group: u32,
        spacing: f64,
        size: f64,
        stroke_width: f64,
        colours: Vec<String>,
        constellation: Option<f64>,
    ) -> Result<Rings, JsValue> {
        let document = web_sys::window()
            .expect("window should exist")
            .document()
            .expect("document should exist");

        let canvas: HtmlCanvasElement = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;

        canvas.set_width(width);
        canvas.set_height(height);

        let ctx = canvas
            .get_context("2d")?
            .expect("canvas should have a 2d context")
            .dyn_into::<CanvasRenderingContext2d>()?;

        Ok(Rings {
            count,
            rings_per_group,
            spacing,
            size,
            width: stroke_width,
            colours,
            constellation,
            canvas,
            ctx,
            time: RefCell::new(0.0),
            hub_phase: js_sys::Math::random() * PI * 2.0,
            centres: RefCell::new(Vec::new()),
        })
    }

    fn hub(&self, time: f64) -> (f64, f64) {
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;

        (
            w / 2.0 + (time * 0.4 + self.hub_phase).sin() * 40.0,
            h / 2.0 + (time * 0.32 + self.hub_phase).cos() * 30.0,
        )
    }

    fn constellation_position(&self, time: f64, group: u32, count: u32, distance: f64) -> (f64, f64) {
        let (hx, hy) = self.hub(time);

        let angle = (PI * 2.0 / count as f64) * group as f64 + time * 0.15;

        (hx + angle.cos() * distance, hy + angle.sin() * distance)
    }

    fn colour(&self, index: u32) -> &str {
        if self.colours.is_empty() {
            return "rgb(255,0,255)";
        }

        &self.colours[(index as usize) % self.colours.len()]
    }
}

impl Operation for Rings {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn execute(
        &self,
        _ctx: &Context,
        _inputs: &[Box<dyn Value>],
    ) -> Result<Vec<Box<dyn Value>>, OperationError> {
        let width = self.canvas.width();
        let height = self.canvas.height();

        {
            let mut time = self.time.borrow_mut();
            *time += 0.03;
        }

        let time = *self.time.borrow();

        {
            let mut centres = self.centres.borrow_mut();

            while (centres.len() as u32) < self.count {
                centres.push(Centre {
                    x: js_sys::Math::random() * width as f64,
                    y: js_sys::Math::random() * height as f64,
                    phase: js_sys::Math::random() * PI * 2.0,
                    speed: 0.5 + js_sys::Math::random(),
                });
            }

            centres.truncate(self.count as usize);
        }

        self.ctx.clear_rect(0.0, 0.0, width as f64, height as f64);
        self.ctx.save();
        self.ctx.set_global_alpha(0.8);

        let zoom = self.size / 30.0;
        let unit = self.spacing * zoom;
        let inner_radius = unit * 0.3;

        let centres = self.centres.borrow();

        for group in 0..self.count {
            let (cx, cy) = if let Some(distance) = self.constellation {
                self.constellation_position(time, group, self.count, distance)
            } else {
                let centre = &centres[group as usize];

                (
                    centre.x + (time * centre.speed + centre.phase).sin() * 50.0,
                    centre.y + (time * centre.speed * 0.8 + centre.phase).cos() * 40.0,
                )
            };

            self.ctx.set_stroke_style_str(self.colour(group));
            self.ctx.set_line_width(self.width);

            for n in 0..=self.rings_per_group {
                let radius = inner_radius + n as f64 * unit;

                self.ctx.begin_path();

                self.ctx
                    .arc(cx, cy, radius, 0.0, PI * 2.0)
                    .map_err(|_| OperationError::WrongValueType)?;

                self.ctx.stroke();
            }
        }

        self.ctx.restore();

        let image_data = self
            .ctx
            .get_image_data(0.0, 0.0, width as f64, height as f64)
            .map_err(|_| OperationError::WrongValueType)?;

        let frame = Frame {
            pixels: image_data.data().0,
            width,
            height,
            timestamp: time,
        };

        Ok(vec![Box::new(frame)])
    }
}
