use crate::compositor::{Context, Operation, OperationError, Value};
use crate::operations::composite::blend_mode::BlendMode;
use crate::operations::{downcast_frame, Frame};

/*
inputs[0] is the foreground (drawn on top), inputs[1] is the
background - matching the stub demo's own wiring order
(vec![keyed_video_source_id, backdrop_source_node_id]). Straight
(non-premultiplied) alpha in, straight alpha out: standard Porter-Duff
"over" alpha accumulation always applies, regardless of which colour
blend mode is chosen - the blend mode only changes how the RGB
channels mix where both frames are opaque, never how alpha itself
accumulates (this mirrors Canvas2D's compositing model, which is what
the eventual JS/WASM boundary will need to match byte-for-byte).
*/
pub struct Compose {
    pub mode: BlendMode,
}

impl Operation for Compose {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn execute(
        &self,
        _ctx: &Context,
        inputs: &[Box<dyn Value>],
    ) -> Result<Vec<Box<dyn Value>>, OperationError> {
        let fg = downcast_frame(inputs.first())?;
        let bg = downcast_frame(inputs.get(1))?;

        if !fg.same_dimensions(bg) {
            return Err(OperationError::DimensionMismatch);
        }

        let mut pixels = Vec::with_capacity(fg.pixels.len());

        for i in (0..fg.pixels.len()).step_by(4) {
            let fg_a = fg.pixels[i + 3] as f32 / 255.0;
            let bg_a = bg.pixels[i + 3] as f32 / 255.0;

            let out_a = fg_a + bg_a * (1.0 - fg_a);

            for c in 0..3 {
                let blended = self.mode.blend_channel(
                    fg.pixels[i + c],
                    bg.pixels[i + c],
                );

                let straight = blended as f32 * fg_a
                    + bg.pixels[i + c] as f32 * (1.0 - fg_a);

                pixels.push(
                    straight.round().clamp(0.0, 255.0) as u8
                );
            }

            pixels.push(
                (out_a * 255.0).round().clamp(0.0, 255.0) as u8
            );
        }

        let frame = Frame {
            pixels,
            width: fg.width,
            height: fg.height,
            timestamp: fg.timestamp,
        };

        Ok(vec![Box::new(frame)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    fn frame(pixels: Vec<u8>) -> Frame {
        Frame {
            pixels,
            width: 1,
            height: 1,
            timestamp: 0.0,
        }
    }

    fn run(mode: BlendMode, fg: Frame, bg: Frame) -> Result<Frame, OperationError> {
        let compose = Compose { mode };
        let ctx = Context { data: Box::new(()) };
        let inputs: Vec<Box<dyn Value>> = vec![Box::new(fg), Box::new(bg)];

        let mut outputs = compose.execute(&ctx, &inputs)?;

        let any_box: Box<dyn Any> = outputs.remove(0);

        Ok(*any_box
            .downcast::<Frame>()
            .expect("compose should output a Frame"))
    }

    #[test]
    fn over_blends_by_straight_alpha() {
        let fg = frame(vec![255, 0, 0, 128]); // semi-transparent red
        let bg = frame(vec![0, 0, 255, 255]); // opaque blue

        let out = run(BlendMode::Over, fg, bg).expect("compose should succeed");

        assert_eq!(out.pixels, vec![128, 0, 127, 255]);
    }

    #[test]
    fn opaque_foreground_is_a_pure_passthrough() {
        let fg = frame(vec![10, 20, 30, 255]);
        let bg = frame(vec![200, 200, 200, 255]);

        let out = run(BlendMode::Over, fg, bg).expect("compose should succeed");

        assert_eq!(out.pixels, vec![10, 20, 30, 255]);
    }

    #[test]
    fn transparent_foreground_is_a_pure_passthrough_of_background() {
        let fg = frame(vec![10, 20, 30, 0]);
        let bg = frame(vec![200, 150, 100, 255]);

        let out = run(BlendMode::Over, fg, bg).expect("compose should succeed");

        assert_eq!(out.pixels, vec![200, 150, 100, 255]);
    }

    #[test]
    fn mismatched_dimensions_error_instead_of_panicking() {
        let fg = frame(vec![255, 0, 0, 255]);
        let bg = Frame {
            pixels: vec![0, 0, 255, 255, 0, 0, 255, 255],
            width: 2,
            height: 1,
            timestamp: 0.0,
        };

        let result = run(BlendMode::Over, fg, bg);

        assert!(matches!(result, Err(OperationError::DimensionMismatch)));
    }

    #[test]
    fn missing_input_errors_instead_of_panicking() {
        let compose = Compose { mode: BlendMode::Over };
        let ctx = Context { data: Box::new(()) };
        let inputs: Vec<Box<dyn Value>> = vec![Box::new(frame(vec![255, 0, 0, 255]))];

        let result = compose.execute(&ctx, &inputs);

        assert!(matches!(result, Err(OperationError::MissingInput)));
    }
}
