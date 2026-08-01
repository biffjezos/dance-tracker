// src/operations/generators/ghost.rs
use std::any::Any;
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

/// Spatial repeat of a masked object: GHOST_COUNT copies, each `n`
/// (1-based) offset by `n * DISTANCE * (SPATIAL_X, SPATIAL_Y)` from the
/// source's own (unmoved) position, all sharing one OPACITY_MULTIPLIER -
/// not a temporal echo/trail despite the name, a spatial one (see
/// ANIMATION_IMPLEMENTATION_PLAN.md's GHOST section for the worked
/// example this came from). Both SOURCE and MASK are required - there is
/// no sensible "no mask wired" behaviour for an operation whose entire
/// job is repeating the masked region.
pub struct Ghost {
    pub ghost_count: usize,
    pub distance: f64,
    pub spatial_x: f64,
    pub spatial_y: f64,
    pub opacity_multiplier: f64,
}

impl Ghost {
    pub fn new() -> Self {
        Self {
            ghost_count: 1,
            distance: 32.0,
            spatial_x: -1.0,
            spatial_y: 0.0,
            opacity_multiplier: 1.0,
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

    /// The full GHOST composite: the source's own cutout at native
    /// opacity, plus `ghost_count` spatially-offset copies at
    /// `opacity_multiplier`, stacked nearest-to-source-on-top (painted
    /// far-to-near - the farthest ghost goes down first, the source
    /// itself goes on top of everything). Stacking order is a judgment
    /// call the user hasn't specified either way - see
    /// ANIMATION_IMPLEMENTATION_PLAN.md's GHOST section.
    pub fn render(&self, source: &[f32], mask: &[f32], width: u32, height: u32) -> Vec<f32> {
        let cutout = Self::cutout_pixels(source, mask);

        let mut result = vec![0f32; cutout.len()];

        for n in (1..=self.ghost_count).rev() {
            let offset_x = n as f64 * self.distance * self.spatial_x;
            let offset_y = n as f64 * self.distance * self.spatial_y;

            let mut ghost = Self::translate_pixels(&cutout, width, height, offset_x, offset_y);
            for px in ghost.chunks_exact_mut(4) {
                px[3] *= self.opacity_multiplier as f32;
            }

            result = Self::composite_over(&ghost, &result);
        }

        Self::composite_over(&cutout, &result)
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
        ]
    }

    fn get_parameter(&self, name: &str) -> Option<Value> {
        match name {
            "GHOST_COUNT" => Some(Value::Number(self.ghost_count as f64)),
            "DISTANCE" => Some(Value::Number(self.distance)),
            "SPATIAL_X" => Some(Value::Number(self.spatial_x)),
            "SPATIAL_Y" => Some(Value::Number(self.spatial_y)),
            "OPACITY_MULTIPLIER" => Some(Value::Number(self.opacity_multiplier)),
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
            (name, _) => Err(OperationError::InvalidParameterType(name.to_string())),
        }
    }

    fn execute(&self, ctx: &Context, inputs: &[(Input, Value)]) -> Result<Vec<Value>, OperationError> {
        let Some(source) = find_input(inputs, Input::Source) else {
            return Err(OperationError::MissingInput("GHOST requires SOURCE".into()));
        };

        let Some(mask) = find_input(inputs, Input::Mask) else {
            return Err(OperationError::MissingInput("GHOST requires MASK".into()));
        };

        let source_image = FloatImage::from_value(source, ctx)?;
        let mask_image = FloatImage::from_value(mask, ctx)?;

        if source_image.width != mask_image.width || source_image.height != mask_image.height {
            return Err(OperationError::InvalidInputType(
                "GHOST's SOURCE and MASK must have matching dimensions".into()
            ));
        }

        let pixels = self.render(&source_image.pixels, &mask_image.pixels, source_image.width, source_image.height);

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
        let out = ghost.render(&source, &mask, 2, 2);
        assert_eq!(out, Ghost::cutout_pixels(&source, &mask));
    }

    #[test]
    fn a_ghost_offsets_by_n_times_distance_along_the_spatial_direction() {
        // One ghost, moving purely in +X, distance 1 - on a 3x1 image, the
        // ghost of pixel 0's content should land on pixel 1.
        let ghost = Ghost { ghost_count: 1, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0, opacity_multiplier: 1.0 };
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
        let out = ghost.render(&source, &mask, 3, 1);

        // Source itself still shows unmoved at x=0.
        assert!((out[3] - 1.0).abs() < 1e-6, "source must render at its own native opacity");
        // The ghost shows red (shifted from x=0) at x=1.
        assert!((out[4] - 1.0).abs() < 1e-6, "expected the ghost's red to land at x=1");
    }

    #[test]
    fn opacity_multiplier_applies_identically_to_every_ghost_not_per_index() {
        let ghost = Ghost { ghost_count: 2, distance: 1.0, spatial_x: 1.0, spatial_y: 0.0, opacity_multiplier: 0.5 };
        let source = vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mask = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let out = ghost.render(&source, &mask, 4, 1);

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
    fn execute_errors_without_a_wired_mask() {
        let ghost = Ghost::new();
        let source = Value::Image(crate::graphics::U8Image::black(2, 2));
        let err = ghost.execute(&context(2, 2), &[(Input::Source, source)]).unwrap_err();
        assert!(matches!(err, OperationError::MissingInput(_)));
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
}
