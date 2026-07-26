/*
Blends two STRAIGHT-alpha colour channel values (0-255), ignoring
alpha entirely - Compose applies this per RGB channel and then handles
the actual over/under alpha accumulation itself, the same split Canvas
2D's globalCompositeOperation draws on internally.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    Over,
    Screen,
    Multiply,
}

impl BlendMode {
    pub fn blend_channel(&self, fg: u8, bg: u8) -> u8 {
        match self {
            BlendMode::Over => fg,

            BlendMode::Screen => {
                255 - (((255 - fg as u32) * (255 - bg as u32)) / 255) as u8
            }

            BlendMode::Multiply => {
                ((fg as u32 * bg as u32) / 255) as u8
            }
        }
    }
}
