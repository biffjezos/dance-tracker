// src/operations/generators/ghost.rs
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::compositor::{
    bbox::Rect,
    Context,
    Operation,
    OperationDescriptor,
    OperationError,
    Input,
    input::{find_bbox, find_input},
    Value,
    metadata::{
        InputDescriptor,
        OperationCategory,
        OperationMetadata,
        OutputKind,
        ParameterDescriptor,
        ParameterKind,
        PIXEL_KINDS,
    },
};

use crate::graphics::FloatImage;

/// Repeat of a masked object, both spatially and in time: ghost `n`
/// (1-based) is offset by `n * DISTANCE * (SPATIAL_X, SPATIAL_Y)` and
/// reads its historical cutout from `n * DELAY` frames back - both scale
/// by ghost index, a cascading chain where ghost `n` is delayed `DELAY`
/// more than ghost `n-1`. OPACITY_MULTIPLIER is different: one shared
/// value for every ghost, not indexed by n. SHOW_SOURCE
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
    /// offset by `n * DISTANCE` and pulled from `n * delay` frames ago
    /// (a cascading chain: ghost `n` is `delay` frames further back than
    /// ghost `n-1`), all at the same `opacity_multiplier`, stacked
    /// nearest-to-source-on-top (painted far-to-near - the spatially-
    /// farthest ghost goes down first). Stacking order is a judgment call the user
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
        self.render_with_cutout(cutout, width, height)
    }

    /// The composite step of `render()`, given an already-computed cutout
    /// (post-mask, pre-translate) - factored out so `execute()`'s
    /// bbox-restricted path (Phase 3 of BBOX_CONVENTIONS.md) can supply a
    /// cutout computed only within the relevant region, without
    /// duplicating this history/translate/composite logic. `render()`
    /// itself is unchanged - still always computes an unrestricted
    /// cutout first, same as before this phase.
    fn render_with_cutout(&self, cutout: Vec<f32>, width: u32, height: u32) -> Vec<f32> {
        self.record_history(&cutout);

        let mut result = vec![0f32; cutout.len()];

        for n in (1..=self.ghost_count).rev() {
            let delayed = self.delayed_cutout(n as u64 * self.delay);
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

    /// The cutout value of a single pixel, computed directly from
    /// `source`/`mask` - identical math to `cutout_pixels`'s own loop
    /// body for that index. Used by `execute()`'s bbox-restricted path.
    fn cutout_single_pixel(source: &[f32], mask: &[f32], x: u32, y: u32, width: u32) -> [f32; 4] {
        let idx = ((y * width + x) * 4) as usize;
        [source[idx], source[idx + 1], source[idx + 2], source[idx + 3] * mask[idx + 3]]
    }

    /// Push this tick's cutout onto the history buffer, then trim it
    /// down to exactly what the deepest ghost currently needs
    /// (`ghost_count * delay` frames back, plus the current one) - never
    /// more, so DELAY/GHOST_COUNT can't grow memory use unboundedly.
    fn record_history(&self, cutout: &[f32]) {
        let mut history = self.history.borrow_mut();
        history.push_back(cutout.to_vec());

        let capacity = (self.ghost_count as u64).saturating_mul(self.delay).saturating_add(1) as usize;
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
            inputs: vec![
                InputDescriptor { kind: Input::Source, accepts: PIXEL_KINDS },
                InputDescriptor { kind: Input::Mask, accepts: PIXEL_KINDS },
            ],
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

        let width = source_image.width;
        let height = source_image.height;

        // Phase 3 of BBOX_CONVENTIONS.md: with a MASK wired, restrict the
        // cutout computation - GHOST's own equivalent of the "processed
        // vs. original" split every other migrated operation expresses
        // via apply_mask - to the intersection of MASK's own reported box
        // and SOURCE's own reported box (no growth: cutout reads only the
        // pixel it writes, no neighbors). SOURCE's box is a valid operand:
        // cutout_pixels is zero-preserving (RGB always copies SOURCE
        // unconditionally, alpha multiplies by MASK's alpha, so a zero
        // SOURCE pixel always yields a zero cutout regardless of MASK).
        //
        // Unlike every other migrated operation, the correct pass-through
        // value outside the work area is literal [0,0,0,0] - NOT SOURCE's
        // own raw pixel. GHOST has no apply_mask call; its cutout's own
        // "untouched" state is fully transparent, not an identity copy.
        // This substitution is still safe: a zero-alpha cutout pixel is
        // always visually inert downstream regardless of its RGB -
        // composite_over's own formula divides by (and thus discards a
        // foreground's RGB contribution entirely whenever) its alpha is
        // 0 - so [0,0,0,0] and "SOURCE's raw pixel with alpha zeroed"
        // produce identical final output.
        //
        // This only restricts the cutout step - the translate/composite
        // loop below still runs over the full frame regardless. Real
        // ghost content can appear anywhere up to
        // GHOST_COUNT * DISTANCE * (SPATIAL_X, SPATIAL_Y) pixels away
        // from MASK's own box, so restricting that loop's own region
        // would require unioning every ghost's own translated box first -
        // a separate, larger change not attempted in this round.
        let cutout = match &mask_pixels {
            Some(mask) => {
                let natural_box = find_bbox(&ctx.input_bboxes, Input::Source)
                    .unwrap_or_else(|| Rect::full(width, height));
                let mask_box = find_bbox(&ctx.input_bboxes, Input::Mask)
                    .unwrap_or_else(|| Rect::full(width, height));
                let work_area = natural_box.intersect(&mask_box);

                let transparent = vec![0f32; source_image.pixels.len()];
                let source_pixels = &source_image.pixels;

                crate::graphics::compute_within_bbox(width, height, work_area, &transparent, |x, y| {
                    Self::cutout_single_pixel(source_pixels, mask, x, y, width)
                })
            }
            None => {
                let opaque_mask = vec![1.0f32; source_image.pixels.len()];
                Self::cutout_pixels(&source_image.pixels, &opaque_mask)
            }
        };

        let pixels = self.render_with_cutout(cutout, width, height);

        Ok(vec![Value::FloatImage(Arc::new(FloatImage {
            pixels,
            width,
            height,
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
    fn each_ghost_delay_scales_by_ghost_index_like_distance_does() {
        // RFC-005 / SPEC-GHOST-DELAY: DELAY must cascade per ghost layer,
        // exactly mirroring DISTANCE's own n-scaling - ghost n reads
        // n * DELAY frames back, not a shared DELAY value.
        let ghost = Ghost { ghost_count: 2, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0, opacity_multiplier: 1.0, delay: 1, show_source: false, ..Ghost::new() };
        let opaque_mask = solid(3, 1, 0.0, 0.0, 0.0, 1.0);

        ghost.render(&solid(3, 1, 1.0, 0.0, 0.0, 1.0), Some(&opaque_mask), 3, 1); // frame 0: red
        ghost.render(&solid(3, 1, 0.0, 1.0, 0.0, 1.0), Some(&opaque_mask), 3, 1); // frame 1: green
        let out = ghost.render(&solid(3, 1, 0.0, 0.0, 1.0, 1.0), Some(&opaque_mask), 3, 1); // frame 2: blue

        // Ghost 1 (x=1) reads 1 * DELAY = 1 frame back from frame 2: green.
        assert!((out[5] - 1.0).abs() < 1e-6, "ghost 1 should show green (1 * delay = 1 frame back), got {:?}", &out[4..8]);
        // Ghost 2 (x=2) reads 2 * DELAY = 2 frames back from frame 2: red.
        assert!((out[8] - 1.0).abs() < 1e-6, "ghost 2 should show red (2 * delay = 2 frames back), got {:?}", &out[8..12]);
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

    fn image(pixels: Vec<u8>, width: u32, height: u32) -> Arc<crate::graphics::U8Image> {
        Arc::new(crate::graphics::U8Image { pixels, width, height, format: crate::graphics::ImageFormat::Rgba8 })
    }

    fn as_u8_pixels(value: &Value) -> Vec<u8> {
        match value {
            Value::FloatImage(out) => out.to_image_clamped(0.0, 1.0).pixels,
            other => panic!("expected a float image, got {:?}", other),
        }
    }

    #[test]
    fn consume_equivalence_a_real_mask_bbox_produces_the_same_output_as_a_full_frame_one() {
        // GHOST_COUNT=0, SHOW_SOURCE=true: render_with_cutout reduces to
        // exactly the cutout itself (composited over an empty result),
        // isolating the cutout-restriction logic from the translate/
        // composite loop for this test.
        let ghost = Ghost { ghost_count: 0, show_source: true, ..Ghost::new() };

        let source = image((0..6).flat_map(|n| [n * 10, n * 10 + 1, n * 10 + 2, 255]).collect(), 6, 1);
        let mask = image(
            vec![
                0, 0, 0, 0,   0, 0, 0, 0,
                0, 0, 0, 255, 0, 0, 0, 255,
                0, 0, 0, 0,   0, 0, 0, 0,
            ],
            6, 1,
        );

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let ctx_with_real_box = Context {
            input_bboxes: vec![
                (Input::Source, Rect::full(6, 1)),
                (Input::Mask, Rect { x0: 2, y0: 0, x1: 4, y1: 1 }),
            ],
            ..context(6, 1)
        };
        let ctx_full_frame = context(6, 1);

        let restricted = ghost.execute(&ctx_with_real_box, &inputs).unwrap();
        let unrestricted = ghost.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn consume_equivalence_holds_even_when_source_itself_reports_a_sub_frame_box() {
        // Verifies cutout_pixels's zero-preservation directly: RGB always
        // copies SOURCE unconditionally and alpha multiplies by MASK's
        // alpha, so cutout([0,0,0,0], mask) is always [0,0,0,0] regardless
        // of MASK - SOURCE's own box is therefore a valid intersection
        // operand here.
        let ghost = Ghost { ghost_count: 0, show_source: true, ..Ghost::new() };

        let mut source_pixels = vec![0u8; 10 * 4];
        for x in 3..7 {
            source_pixels[x * 4..x * 4 + 4].copy_from_slice(&[100, 150, 200, 255]);
        }
        let source = image(source_pixels, 10, 1);
        let mask = image(vec![255; 10 * 4], 10, 1);

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let ctx_with_real_source_box = Context {
            input_bboxes: vec![
                (Input::Source, Rect { x0: 3, y0: 0, x1: 7, y1: 1 }),
                (Input::Mask, Rect::full(10, 1)),
            ],
            ..context(10, 1)
        };
        let ctx_full_frame = context(10, 1);

        let restricted = ghost.execute(&ctx_with_real_source_box, &inputs).unwrap();
        let unrestricted = ghost.execute(&ctx_full_frame, &inputs).unwrap();

        assert_eq!(as_u8_pixels(&restricted[0]), as_u8_pixels(&unrestricted[0]));
    }

    #[test]
    fn a_ghost_translated_outside_masks_own_box_still_renders_correctly() {
        // The load-bearing test for GHOST specifically: only the cutout
        // step is restricted to MASK's own box - the translate/composite
        // loop still runs full-frame, so a ghost translated well outside
        // MASK's own reported box must still show up correctly. This is
        // the exact risk the cutout-only restriction must not introduce.
        let ghost = Ghost {
            ghost_count: 1,
            distance: 5.0,
            spatial_x: 1.0,
            spatial_y: 0.0,
            opacity_multiplier: 1.0,
            delay: 0,
            show_source: false,
            ..Ghost::new()
        };

        // Opaque red at x=0 only; MASK is opaque exactly there too, and
        // MASK's own reported box is a tight [0,1) around it - nowhere
        // near where the ghost (offset +5) will land.
        let mut source_pixels = vec![0u8; 10 * 4];
        source_pixels[0..4].copy_from_slice(&[255, 0, 0, 255]);
        let source = image(source_pixels, 10, 1);
        let mut mask_pixels = vec![0u8; 10 * 4];
        mask_pixels[3] = 255;
        let mask = image(mask_pixels, 10, 1);

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let ctx_with_tight_mask_box = Context {
            input_bboxes: vec![
                (Input::Source, Rect::full(10, 1)),
                (Input::Mask, Rect { x0: 0, y0: 0, x1: 1, y1: 1 }),
            ],
            ..context(10, 1)
        };

        let values = ghost.execute(&ctx_with_tight_mask_box, &inputs).unwrap();
        let pixels = as_u8_pixels(&values[0]);

        // The ghost must land at x=5 (0 + 1*5*1), well outside MASK's own
        // [0,1) box - real, opaque red content, not skipped/transparent.
        assert_eq!(&pixels[5 * 4..5 * 4 + 4], &[255, 0, 0, 255], "the translated ghost must still render outside MASK's own reported box");
    }

    #[test]
    fn a_smaller_mask_bbox_computes_strictly_fewer_pixels_than_a_full_frame_one() {
        use crate::graphics::mask::{reset_pixels_computed, take_pixels_computed};

        let ghost = Ghost { ghost_count: 0, show_source: true, ..Ghost::new() };

        let source = image((0..16).flat_map(|n| [n, n, n, 255]).collect(), 4, 4);
        let mask = image(vec![255; 4 * 4 * 4], 4, 4);

        let inputs = [
            (Input::Source, Value::Image(source)),
            (Input::Mask, Value::Image(mask)),
        ];

        let small_box_ctx = Context {
            input_bboxes: vec![
                (Input::Source, Rect::full(4, 4)),
                (Input::Mask, Rect { x0: 1, y0: 1, x1: 2, y1: 2 }),
            ],
            ..context(4, 4)
        };
        reset_pixels_computed();
        ghost.execute(&small_box_ctx, &inputs).unwrap();
        let small_box_pixels = take_pixels_computed().expect("GHOST with a wired MASK must record a pixel count");

        let full_frame_ctx = context(4, 4);
        reset_pixels_computed();
        ghost.execute(&full_frame_ctx, &inputs).unwrap();
        let full_frame_pixels = take_pixels_computed().expect("GHOST with a wired MASK must record a pixel count");

        assert_eq!(small_box_pixels, 1);
        assert_eq!(full_frame_pixels, 16);
        assert!(small_box_pixels < full_frame_pixels);
    }

    #[test]
    fn checkerboard_resize_move_geometric_mask_end_to_end_matches_with_bbox_consumption_on_or_off() {
        use crate::compositor::graph::Graph;
        use crate::compositor::executors::{Execute, PreviewExecutor, RenderExecutor};
        use crate::graphics::Color;
        use crate::operations::generators::Checkerboard;
        use crate::operations::sources::ImageSource;
        use crate::operations::transform::{Move, Resize};

        let mut graph = Graph::new(4, 4);

        let mut source = ImageSource::new();
        source.set_image(image((0..16).flat_map(|n| [n * 15, 0, 0, 255]).collect(), 4, 4));
        let source_id = graph.add_node(Box::new(source));

        let mut checkerboard = Checkerboard::new();
        checkerboard.color_a = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        checkerboard.color_b = checkerboard.color_a;
        let checkerboard_id = graph.add_node(Box::new(checkerboard));

        let mut resize = Resize::new();
        resize.set_parameter("SCALE_X", Value::Number(50.0)).unwrap();
        resize.set_parameter("SCALE_Y", Value::Number(50.0)).unwrap();
        let resize_id = graph.add_node(Box::new(resize));
        graph.connect(resize_id, Input::Source, checkerboard_id).unwrap();

        let move_id = graph.add_node(Box::new(Move::new()));
        graph.connect(move_id, Input::Source, resize_id).unwrap();

        let ghost = Ghost { ghost_count: 0, show_source: true, ..Ghost::new() };
        let ghost_id = graph.add_node(Box::new(ghost));
        graph.connect(ghost_id, Input::Source, source_id).unwrap();
        graph.connect(ghost_id, Input::Mask, move_id).unwrap();

        graph.validate().expect("the wired pipeline is valid");

        let ctx = context(4, 4);

        let on_values = RenderExecutor::new().execute(&graph, ghost_id, &ctx).unwrap();
        let on_pixels = as_u8_pixels(&on_values[0]);

        let source_value = PreviewExecutor::default().execute(&graph, source_id, &ctx).unwrap().into_iter().next().unwrap();
        let mask_value = PreviewExecutor::default().execute(&graph, move_id, &ctx).unwrap().into_iter().next().unwrap();

        let ghost_off = Ghost { ghost_count: 0, show_source: true, ..Ghost::new() };
        let off_values = ghost_off.execute(&ctx, &[
            (Input::Source, source_value),
            (Input::Mask, mask_value),
        ]).unwrap();
        let off_pixels = as_u8_pixels(&off_values[0]);

        assert_eq!(on_pixels, off_pixels, "bbox consumption on vs off must produce pixel-identical output");
    }
}
