#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// "#rrggbb" - the wire format a Color-kind parameter is shown and set
    /// as, matching what a browser's native <input type="color"> produces.
    pub fn to_hex(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    pub fn from_hex(hex: &str) -> Option<Color> {
        let hex = hex.strip_prefix('#')?;
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        })
    }

    /// This color as RGBA8, for writing directly into a pixel buffer.
    pub fn to_rgba_u8(&self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }

    /// (hue in 0..360 degrees, saturation in 0..1, value in 0..1). The one
    /// canonical RGB->HSV formula - both RGB TO HSV's per-pixel conversion
    /// and HUE KEY's own key-color-to-target-hue extraction go through
    /// this, rather than each reimplementing it.
    pub fn to_hsv(&self) -> (f64, f64, f64) {
        let r = self.r as f64;
        let g = self.g as f64;
        let b = self.b as f64;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;
        let s = if max == 0.0 { 0.0 } else { delta / max };

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta).rem_euclid(6.0))
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        (h, s, v)
    }
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn to_hex_and_from_hex_round_trip_through_the_wire_format() {
        let red = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        assert_eq!(red.to_hex(), "#ff0000");
        assert_eq!(Color::from_hex("#ff0000").unwrap().r, 1.0);
    }

    #[test]
    fn from_hex_rejects_malformed_input() {
        assert!(Color::from_hex("ff0000").is_none());
        assert!(Color::from_hex("#ff00").is_none());
        assert!(Color::from_hex("#gggggg").is_none());
    }

    #[test]
    fn to_hsv_of_primary_colours() {
        let red = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        let green = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
        let blue = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };

        assert_eq!(red.to_hsv(), (0.0, 1.0, 1.0));
        assert_eq!(green.to_hsv(), (120.0, 1.0, 1.0));
        assert_eq!(blue.to_hsv(), (240.0, 1.0, 1.0));
    }

    #[test]
    fn to_hsv_of_black_white_and_grey_has_zero_saturation() {
        let black = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
        let white = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let grey = Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 };

        assert_eq!(black.to_hsv(), (0.0, 0.0, 0.0));
        assert_eq!(white.to_hsv(), (0.0, 0.0, 1.0));
        assert_eq!(grey.to_hsv(), (0.0, 0.0, 0.5));
    }

    #[test]
    fn to_hsv_hue_never_goes_negative() {
        // Magenta: max is r and b tied, g is lowest - exercises the branch
        // most likely to produce a negative angle before wrapping.
        let magenta = Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
        let (h, _, _) = magenta.to_hsv();
        assert!((0.0..360.0).contains(&h), "expected hue in 0..360, got {}", h);
        assert_eq!(h, 300.0);
    }
}