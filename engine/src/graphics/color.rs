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
}