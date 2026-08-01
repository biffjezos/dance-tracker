// src/operations/generators/ghost.rs
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::compositor::{
    Context,
    Operation,
    OperationDescriptor,
    OperationError,
    Input,
    input::find_input,
    Value,
    metadata::{
        OperationCategory,
        OperationMetadata,
        OutputKind,
        ParameterDescriptor,
        ParameterKind,
    },
};

use crate::graphics::FloatImage;

/// Repeat of a masked object, both spatially and in time: ghost `n`
/// (1-based) is offset by `n * DISTANCE * (SPATIAL_X, SPATIAL_Y)` - that
/// part scales by ghost index. DELAY does not: every ghost shows the
/// source from the same DELAY frames ago, same as OPACITY_MULTIPLIER is
/// one shared value for every ghost, not indexed by n. SHOW_SOURCE
/// toggles whether the live (unmoved, undelayed) source itself is
/// composited on top at all, or only the ghost trail shows. Only SOURCE
/// is required - MASK is optional: when it's not wired, SOURCE's own
/// alpha channel is used as the cutout boundary directly (equivalent to
/// a mask that's fully opaque everywhere), for a SOURCE that already
/// carries a meaningful alpha channel of its own.
pub struct Ghost {
    pub ghost_count: usize,
    pub distance: f64,
    pub spatial_x: f64,
    pub spatial_y: f64,
    pub opacity_multiplier: f64,
    pub delay: u64,
    pub show_source: bool,
    // Interior mutability: `Operation::execute()` only ever gets `&self`
    // (see ANIMATION_CONVENTIONS.md - operations stay pure functions of
    // their own params + resolved inputs from the outside), but DELAY
    // inherently needs to remember past frames. Same pattern this
    // codebase's own test doubles already use (`Cell<f64>` in
    // executors/render.rs's tests) for &self-compatible mutation, just
    // in a real operation instead of a stub. Holds cutouts (post-mask,
    // pre-translate) in oldest-first order; capacity trimmed to exactly
    // what the deepest ghost currently needs.
    history: RefCell<VecDeque<Vec<f32>>>,
}

impl Ghost {
    pub fn new() -> Self {
        Self {
            ghost_count: 1,
            distance: 32.0,
            spatial_x: -1.0,
            spatial_y: 0.0,
            opacity_multiplier: 1.0,
            delay: 0,
            show_source: true,
            history: RefCell::new(VecDeque::new()),
        }
    }

    /// Isolate the masked object as its own standalone RGBA buffer:
    /// source colour, alpha = source alpha x mask alpha (mask's alpha
    /// channel is the coverage weight, same convention `apply_mask`
    /// already uses for MASK inputs elsewhere). Transparent wherever the
    /// mask doesn't cover. Not the same thing as `apply_mask` - that
    /// blends two already-computed *results* toward each other by mask
    /// weight; this extracts a region as a standalone image.
    pub fn cutout_pixels(source: &[f32], mask: &[f32]) -> Vec<f32> {
        let mut output = vec![0f32; source.len()];

        for ((src, msk), out) in source
            .chunks_exact(4)
            .zip(mask.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            out[0] = src[0];
            out[1] = src[1];
            out[2] = src[2];
            out[3] = src[3] * msk[3];
        }

        output
    }

    /// Shift an RGBA buffer by `(offset_x, offset_y)` pixels - nearest-
    /// neighbor, transparent padding at the vacated edge. Same
    /// inverse-mapping shape as `Resize::resize_pixels`, a translation
    /// instead of a scale.
    pub fn translate_pixels(pixels: &[f32], width: u32, height: u32, offset_x: f64, offset_y: f64) -> Vec<f32> {
        let mut output = vec![0f32; pixels.len()];

        for y in 0..height {
            for x in 0..width {
                let src_x = x as f64 - offset_x;
                let src_y = y as f64 - offset_y;

                if src_x < 0.0 || src_y < 0.0 || src_x >= width as f64 || src_y >= height as f64 {
                    continue;
                }

                let sx = src_x.round() as u32;
                let sy = src_y.round() as u32;
                if sx >= width || sy >= height {
                    continue;
                }

                let dest_index = ((y * width + x) * 4) as usize;
                let src_index = ((sy * width + sx) * 4) as usize;
                output[dest_index..dest_index + 4].copy_from_slice(&pixels[src_index..src_index + 4]);
            }
        }

        output
    }

    /// Standard straight-alpha (not premultiplied) Porter-Duff "over":
    /// `fg` composited on top of `bg`. Deliberately its own explicit
    /// helper, not a retrofit of Add/Multiply/Screen's existing
    /// uniform-4-channel semantics (see PIXEL_CONVENTIONS.md) - alpha
    /// gets its own dedicated formula here, it is not treated like
    /// another colour channel. No extra gamut-safety logic needed: this
    /// is a convex combination weighted by alpha in 0..1, so it cannot
    /// introduce an out-of-gamut RGB value on its own.
    pub fn composite_over(fg: &[f32], bg: &[f32]) -> Vec<f32> {
        let mut output = vec![0f32; fg.len()];

        for ((fg_px, bg_px), out_px) in fg
            .chunks_exact(4)
            .zip(bg.chunks_exact(4))
            .zip(output.chunks_exact_mut(4))
        {
            let fg_a = fg_px[3];
            let bg_a = bg_px[3];
            let out_a = fg_a + bg_a * (1.0 - fg_a);

            for c in 0..3 {
                out_px[c] = if out_a > 0.0 {
                    (fg_px[c] * fg_a + bg_px[c] * bg_a * (1.0 - fg_a)) / out_a
                } else {
                    0.0
                };
            }

            out_px[3] = out_a;
        }

        output
    }

    /// The full GHOST composite: `ghost_count` copies, each spatially
    /// offset by `n * DISTANCE`, all pulled from the same `delay` frames
    /// ago and all at the same `opacity_multiplier`, stacked nearest-to-
    /// source-on-top (painted far-to-near - the spatially-farthest ghost
    /// goes down first). Stacking order is a judgment call the user
    /// hasn't specified either way - see ANIMATION_IMPLEMENTATION_PLAN.md's
    /// GHOST section. The live source itself is composited on top last,
    /// at its own native opacity, only when `show_source` is true.
    ///
    /// Records the current frame's cutout into `history` on every call -
    /// callers that need DELAY to mean real elapsed frames (not "frames
    /// this operation happened to be re-evaluated on") must keep this
    /// operation live (see `is_live()` below) so the render loop never
    /// skips a tick's call here.
    ///
    /// `mask` is optional: pass `None` to use `source`'s own alpha channel
    /// as-is (a fully-opaque stand-in mask), for a SOURCE that already
    /// carries a meaningful alpha of its own with no separate MASK wired.
    pub fn render(&self, source: &[f32], mask: Option<&[f32]>, width: u32, height: u32) -> Vec<f32> {
        let opaque_mask;
        let mask = match mask {
            Some(mask) => mask,
            None => {
                opaque_mask = vec![1.0f32; source.len()];
                &opaque_mask
            }
        };
        let cutout = Self::cutout_pixels(source, mask);
        self.record_history(&cutout);

        let mut result = vec![0f32; cutout.len()];

        for n in (1..=self.ghost_count).rev() {
            let delayed = self.delayed_cutout(self.delay);
            let offset_x = n as f64 * self.distance * self.spatial_x;
            let offset_y = n as f64 * self.distance * self.spatial_y;

            let mut ghost = Self::translate_pixels(&delayed, width, height, offset_x, offset_y);
            for px in ghost.chunks_exact_mut(4) {
                px[3] *= self.opacity_multiplier as f32;
            }

            result = Self::composite_over(&ghost, &result);
        }

        if self.show_source {
            Self::composite_over(&cutout, &result)
        } else {
            result
        }
    }

    /// Push this tick's cutout onto the history buffer, then trim it
    /// down to exactly what the deepest ghost currently needs
    /// (`ghost_count * delay` frames back, plus the current one) - never
    /// more, so DELAY/GHOST_COUNT can't grow memory use unboundedly.
    fn record_history(&self, cutout: &[f32]) {
        let mut history = self.history.borrow_mut();
        history.push_back(cutout.to_vec());

        let capacity = self.delay as usize + 1;
        while history.len() > capacity {
            history.pop_front();
        }
    }

    /// The cutout from `frames_back` frames ago, clamped to the oldest
    /// frame actually available - e.g. DELAY=5 on the graph's 2nd tick
    /// shows the 1st (oldest available) frame rather than nothing, since
    /// there's no real "before the graph started" content to show.
    /// `record_history` always runs first in `render()`, so `history` is
    /// never empty here.
    fn delayed_cutout(&self, frames_back: u64) -> Vec<f32> {
        let history = self.history.borrow();
        let last = history.len() - 1;
        let index = last.saturating_sub(frames_back as usize);
        history[index].clone()
    }
}

impl Default for Ghost {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation for Ghost {
    fn descriptor(&self) -> OperationDescriptor {
        OperationDescriptor {
            id: "ghost",
            menu: "GENERATE",
            label: "GHOST",
            action: None,
            ui_action: None,
            create_node: Some("ghost"),
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
            display_name: "Ghost",
            category: OperationCategory::Composite,
            inputs: vec![Input::Source, Input::Mask],
            outputs: vec![OutputKind::FloatImage],
        }
    }

    fn parameters(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor {
                name: "GHOST_COUNT",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "DISTANCE",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "SPATIAL_X",
                kind: ParameterKind::Number { step: 0.1, min: None, max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "SPATIAL_Y",
                kind: ParameterKind::Number { step: 0.1, min: None, max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "OPACITY_MULTIPLIER",
                kind: ParameterKind::Number { step: 0.05, min: Some(0.0), max: Some(1.0) },
                group: None,
            },
            ParameterDescriptor {
                name: "DELAY",
                kind: ParameterKind::Number { step: 1.0, min: Some(0.0), max: None },
                group: None,
            },
            ParameterDescriptor {
                name: "SHOW_SOURCE",
                kind: ParameterKind::Boolean,
                group: None,
            },
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "GHOST_COUNT" => Some(Value::Number(self.ghost_count as f64)),
            "DISTANCE" => Some(Value::Number(self.distance)),
            "SPATIAL_X" => Some(Value::Number(self.spatial_x)),
            "SPATIAL_Y" => Some(Value::Number(self.spatial_y)),
            "OPACITY_MULTIPLIER" => Some(Value::Number(self.opacity_multiplier)),
            "DELAY" => Some(Value::Number(self.delay as f64)),
            "SHOW_SOURCE" => Some(Value::Boolean(self.show_source)),
            _ => None,
        }
    }

    fn set_parameter(&mut self, name: &str, value: Value) -> Result<(), OperationError> {
        match (name, value) {
            ("GHOST_COUNT", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.ghost_count = v.round() as usize;
                Ok(())
            }
            ("DISTANCE", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.distance = v;
                Ok(())
            }
            ("SPATIAL_X", Value::Number(v)) => {
                self.spatial_x = v;
                Ok(())
            }
            ("SPATIAL_Y", Value::Number(v)) => {
                self.spatial_y = v;
                Ok(())
            }
            ("OPACITY_MULTIPLIER", Value::Number(v)) => {
                if !(0.0..=1.0).contains(&v) {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.opacity_multiplier = v;
                Ok(())
            }
            ("DELAY", Value::Number(v)) => {
                if v < 0.0 {
                    return Err(OperationError::InvalidParameterValue(name.to_string(), v.to_string()));
                }
                self.delay = v.round() as u64;
                Ok(())
            }
            ("SHOW_SOURCE", Value::Boolean(v)) => {
                self.show_source = v;
                Ok(())
            }
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    // DELAY needs to see every real render tick to mean "N frames ago" -
    // if the cross-tick cache (RenderExecutor) ever skipped calling
    // execute() here, the history buffer would fall out of sync with
    // actual elapsed frames. Always re-executing costs little (GHOST's
    // own math is cheap, and its SOURCE is typically already live).
    fn is_live(&self) -> bool {
        true
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(source) = find_input(inputs, Input::Source) else {
            return Err(OperationError::MissingInput("GHOST requires SOURCE".into()));
        };

        let source_image = FloatImage::from_value(source, ctx)?;

        let mask_pixels = match find_input(inputs, Input::Mask) {
            Some(mask) => {
                let mask_image = FloatImage::from_value(mask, ctx)?;
                if source_image.width != mask_image.width || source_image.height != mask_image.height {
                    return Err(OperationError::InvalidInputType(
                        "GHOST's SOURCE and MASK must have matching dimensions".into()
                    ));
                }
                Some(mask_image.pixels)
            }
            None => None,
        };

        let pixels = self.render(&source_image.pixels, mask_pixels.as_deref(), source_image.width, source_image.height);

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels,
            width: source_image.width,
            height: source_image.height,
        }))])
    }
}

inventory::submit! {
    crate::operations::inventory::OperationInfo {
        constructor: || Box::new(Ghost::new())
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

    fn solid(width: u32, height: u32, r: f32, g: f32, b: f32, a: f32) -> Vec<f32> {
        (0..(width as usize * height as usize)).flat_map(|_| [r, g, b, a]).collect()
    }

    #[test]
    fn cutout_keeps_colour_and_multiplies_alpha_by_the_mask() {
        let source = vec![0.5, 0.6, 0.7, 1.0];
        let mask = vec![0.0, 0.0, 0.0, 0.5];
        let cutout = Ghost::cutout_pixels(&source, &mask);
        assert_eq!(cutout, vec![0.5, 0.6, 0.7, 0.5]);
    }

    #[test]
    fn cutout_is_fully_transparent_outside_the_mask() {
        let source = vec![1.0, 1.0, 1.0, 1.0];
        let mask = vec![0.0, 0.0, 0.0, 0.0];
        let cutout = Ghost::cutout_pixels(&source, &mask);
        assert_eq!(cutout[3], 0.0);
    }

    #[test]
    fn translate_shifts_content_and_pads_the_vacated_edge_with_transparency() {
        // 2x1 image: opaque red then opaque blue.
        let pixels = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0];
        let shifted = Ghost::translate_pixels(&pixels, 2, 1, 1.0, 0.0);

        // Pixel 0 now samples from off the left edge - transparent.
        assert_eq!(&shifted[0..4], &[0.0, 0.0, 0.0, 0.0]);
        // Pixel 1 now shows what used to be at pixel 0 (red).
        assert_eq!(&shifted[4..8], &[1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn composite_over_a_transparent_background_reproduces_the_foreground() {
        let fg = vec![0.2, 0.4, 0.6, 0.8];
        let bg = vec![0.0, 0.0, 0.0, 0.0];
        let out = Ghost::composite_over(&fg, &bg);
        assert!((out[0] - 0.2).abs() < 1e-6);
        assert!((out[3] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn composite_over_a_fully_opaque_foreground_hides_the_background() {
        let fg = vec![1.0, 0.0, 0.0, 1.0];
        let bg = vec![0.0, 1.0, 0.0, 1.0];
        let out = Ghost::composite_over(&fg, &bg);
        assert_eq!(out, vec![1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn zero_ghosts_renders_just_the_masked_source() {
        let ghost = Ghost { ghost_count: 0, ..Ghost::new() };
        let source = solid(2, 2, 1.0, 0.0, 0.0, 1.0);
        let mask = solid(2, 2, 0.0, 0.0, 0.0, 1.0);
        let out = ghost.render(&source, Some(&mask), 2, 2);
        assert_eq!(out, Ghost::cutout_pixels(&source, &mask));
    }

    #[test]
    fn a_ghost_offsets_by_n_times_distance_along_the_spatial_direction() {
        // One ghost, moving purely in +X, distance 1 - on a 3x1 image, the
        // ghost of pixel 0's content should land on pixel 1.
        let ghost = Ghost { ghost_count: 1, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0, opacity_multiplier: 1.0, ..Ghost::new() };
        let source = vec![
            1.0, 0.0, 0.0, 1.0, // x=0: opaque red
            0.0, 0.0, 0.0, 0.0, // x=1: transparent
            0.0, 0.0, 0.0, 0.0, // x=2: transparent
        ];
        let mask = vec![
            0.0, 0.0, 0.0, 1.0,
            0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0,
        ];
        let out = ghost.render(&source, Some(&mask), 3, 1);

        // Source itself still shows unmoved at x=0.
        assert!((out[3] - 1.0).abs() < 1e-6, "source must render at its own native opacity");
        // The ghost shows red (shifted from x=0) at x=1.
        assert!((out[4] - 1.0).abs() < 1e-6, "expected the ghost's red to land at x=1");
    }

    #[test]
    fn opacity_multiplier_applies_identically_to_every_ghost_not_per_index() {
        let ghost = Ghost { ghost_count: 2, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0, opacity_multiplier: 0.5, ..Ghost::new() };
        let source = vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mask = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let out = ghost.render(&source, Some(&mask), 4, 1);

        // Ghost 1 (nearest, x=1) and ghost 2 (farthest, x=2) must carry the
        // exact same opacity multiplier - not a progressively fading value.
        assert!((out[7] - 0.5).abs() < 1e-6, "ghost 1's alpha");
        assert!((out[11] - 0.5).abs() < 1e-6, "ghost 2's alpha must equal ghost 1's, not fade further");
    }

    #[test]
    fn execute_errors_without_a_wired_source() {
        let ghost = Ghost::new();
        let mask = Value::Image(crate::graphics::U8Image::black(2, 2));
        let err = ghost.execute(&context(2, 2), &[(Input::Mask, mask)]).unwrap_err();
        assert!(matches!(err, OperationError::MissingInput(_)));
    }

    #[test]
    fn execute_succeeds_without_a_wired_mask_using_sources_own_alpha() {
        let ghost = Ghost { ghost_count: 0, ..Ghost::new() };
        let source = Value::Image(crate::graphics::U8Image::black(2, 2));
        let values = ghost.execute(&context(2, 2), &[(Input::Source, source)]).unwrap();

        match &values[0] {
            Value::FloatImage(out) => {
                assert!((out.pixels[3] - 1.0).abs() < 1e-6, "expected SOURCE's own opaque alpha to pass through unchanged with no MASK wired");
            }
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn render_without_a_mask_matches_rendering_with_a_fully_opaque_one() {
        let ghost = Ghost { ghost_count: 1, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0, opacity_multiplier: 1.0, ..Ghost::new() };
        let source = vec![1.0, 0.0, 0.0, 0.6, 0.0, 1.0, 0.0, 0.3];
        let opaque_mask = solid(2, 1, 0.0, 0.0, 0.0, 1.0);

        let without_mask = ghost.render(&source, None, 2, 1);
        let matching_ghost = Ghost { ghost_count: 1, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0, opacity_multiplier: 1.0, ..Ghost::new() };
        let with_opaque_mask = matching_ghost.render(&source, Some(&opaque_mask), 2, 1);

        assert_eq!(without_mask, with_opaque_mask, "no MASK wired should behave exactly like a fully-opaque one");
    }

    #[test]
    fn execute_errors_on_mismatched_dimensions() {
        let ghost = Ghost::new();
        let source = Value::Image(crate::graphics::U8Image::black(4, 4));
        let mask = Value::Image(crate::graphics::U8Image::black(2, 2));
        let err = ghost
            .execute(&context(4, 4), &[(Input::Source, source), (Input::Mask, mask)])
            .unwrap_err();
        assert!(matches!(err, OperationError::InvalidInputType(_)));
    }

    #[test]
    fn set_parameter_rejects_an_opacity_multiplier_above_one() {
        let mut ghost = Ghost::new();
        let err = ghost.set_parameter("OPACITY_MULTIPLIER", Value::Number(1.5)).unwrap_err();
        assert!(matches!(err, OperationError::InvalidParameterValue(_, _)));
    }

    #[test]
    fn is_live_returns_true() {
        assert!(Ghost::new().is_live(), "DELAY must see every real tick or it falls out of sync with elapsed frames");
    }

    #[test]
    fn zero_delay_uses_the_current_frame_same_as_before_delay_existed() {
        let ghost = Ghost { ghost_count: 1, distance: 0.0, spatial_x: 0.0, spatial_y: 0.0, opacity_multiplier: 1.0, delay: 0, ..Ghost::new() };
        let red = solid(1, 1, 1.0, 0.0, 0.0, 1.0);
        let opaque_mask = solid(1, 1, 0.0, 0.0, 0.0, 1.0);

        ghost.render(&red, Some(&opaque_mask), 1, 1);
        let green = solid(1, 1, 0.0, 1.0, 0.0, 1.0);
        let out = ghost.render(&green, Some(&opaque_mask), 1, 1);

        // With DELAY=0 the ghost tracks the current frame - green, not
        // the earlier red frame.
        assert!((out[1] - 1.0).abs() < 1e-6, "expected the ghost to show the current (green) frame, got {:?}", &out[0..4]);
    }

    #[test]
    fn delay_shows_an_older_frame_from_history() {
        let ghost = Ghost { ghost_count: 1, distance: 0.0, spatial_x: 0.0, spatial_y: 0.0, opacity_multiplier: 1.0, delay: 2, ..Ghost::new() };
        let opaque_mask = solid(1, 1, 0.0, 0.0, 0.0, 1.0);

        // Three ticks: red, green, blue. With DELAY=2 (and zero spatial
        // offset, so the ghost lands exactly on the same pixel as the
        // live source), the 3rd tick's ghost should show frame 1 (red),
        // 2 frames behind the current (blue) frame.
        ghost.render(&solid(1, 1, 1.0, 0.0, 0.0, 1.0), Some(&opaque_mask), 1, 1); // frame 0: red
        ghost.render(&solid(1, 1, 0.0, 1.0, 0.0, 1.0), Some(&opaque_mask), 1, 1); // frame 1: green
        let out = ghost.render(&solid(1, 1, 0.0, 0.0, 1.0, 1.0), Some(&opaque_mask), 1, 1); // frame 2: blue

        // show_source defaults true, so the top layer is the live (blue)
        // source - the ghost is fully covered here since both land on
        // the same pixel with a fully opaque source on top. Turn source
        // off to actually see the delayed ghost's own colour.
        let _ = out;

        let hidden_source_ghost = Ghost { show_source: false, ..ghost };
        let out = hidden_source_ghost.render(&solid(1, 1, 0.0, 0.0, 1.0, 1.0), Some(&opaque_mask), 1, 1);
        // history already has [red, green, blue, blue] at this point;
        // delay=2 back from the just-pushed 4th entry (blue) lands on
        // green (index 1) - proves DELAY pulls an older frame rather
        // than the current one.
        assert!((out[1] - 1.0).abs() < 1e-6, "expected an older (green) frame, got {:?}", &out[0..4]);
    }

    #[test]
    fn every_ghost_uses_the_same_delay_not_scaled_by_ghost_index() {
        // Regression: DELAY must behave like OPACITY_MULTIPLIER (one
        // shared value for every ghost), not like DISTANCE (scaled by n).
        let ghost = Ghost { ghost_count: 2, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0, opacity_multiplier: 1.0, delay: 1, show_source: false, ..Ghost::new() };
        let opaque_mask = solid(3, 1, 0.0, 0.0, 0.0, 1.0);

        ghost.render(&solid(3, 1, 1.0, 0.0, 0.0, 1.0), Some(&opaque_mask), 3, 1); // frame 0: red
        let out = ghost.render(&solid(3, 1, 0.0, 1.0, 0.0, 1.0), Some(&opaque_mask), 3, 1); // frame 1: green

        // Both ghost 1 (x=1) and ghost 2 (x=2) must show the same
        // (delayed, red) frame - not ghost 2 reaching back twice as far.
        assert!((out[4] - 1.0).abs() < 1e-6, "ghost 1 should show red (delay=1 back)");
        assert!((out[8] - 1.0).abs() < 1e-6, "ghost 2 should also show red, the same delay as ghost 1 - not a further-back frame");
    }

    #[test]
    fn delay_clamps_to_the_oldest_available_frame_before_enough_history_exists() {
        let ghost = Ghost { ghost_count: 1, distance: 0.0, spatial_x: 0.0, spatial_y: 0.0, opacity_multiplier: 1.0, delay: 10, show_source: false, ..Ghost::new() };
        let opaque_mask = solid(1, 1, 0.0, 0.0, 0.0, 1.0);

        // Only one frame of history exists (this call itself) - DELAY=10
        // must clamp to it rather than panic or show nothing.
        let out = ghost.render(&solid(1, 1, 1.0, 0.0, 0.0, 1.0), Some(&opaque_mask), 1, 1);
        assert!((out[0] - 1.0).abs() < 1e-6, "expected the only available (red) frame, got {:?}", &out[0..4]);
    }

    #[test]
    fn show_source_false_hides_the_live_source_leaving_only_ghosts() {
        let ghost = Ghost { ghost_count: 1, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0, opacity_multiplier: 1.0, show_source: false, ..Ghost::new() };
        let source = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        let mask = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];

        let out = ghost.render(&source, Some(&mask), 2, 1);

        // x=0 (where only the live source had content) must now be
        // transparent - the source layer is hidden.
        assert_eq!(&out[0..4], &[0.0, 0.0, 0.0, 0.0]);
        // x=1 (where the ghost landed) still shows the ghost's red.
        assert!((out[4] - 1.0).abs() < 1e-6);
    }
}
