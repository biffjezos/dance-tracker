/*
Chroma and difference keying share everything except how they get
their per-pixel reference colour (a fixed key colour vs. another
frame's pixel at the same position) - Fill and the "is this pixel far
enough from the reference to count as foreground" math live here once,
both operations call into it.
*/

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    Solid(u8, u8, u8),
    Video,
}

/*
Ports the exact math from the old JS Segmentation.process(): a pixel
is foreground (kept, opaque) when the sum of per-channel absolute
differences against the reference colour exceeds threshold; otherwise
it's background (fully transparent). Kept pixels show either a flat
Fill::Solid colour or the video's own colour (Fill::Video).
*/
pub fn key_pixel(
    video_rgb: (u8, u8, u8),
    reference_rgb: (u8, u8, u8),
    threshold: u32,
    fill: Fill,
) -> (u8, u8, u8, u8) {
    let diff = (video_rgb.0 as i32 - reference_rgb.0 as i32).unsigned_abs()
        + (video_rgb.1 as i32 - reference_rgb.1 as i32).unsigned_abs()
        + (video_rgb.2 as i32 - reference_rgb.2 as i32).unsigned_abs();

    if diff <= threshold {
        return (0, 0, 0, 0);
    }

    match fill {
        Fill::Solid(r, g, b) => (r, g, b, 255),
        Fill::Video => (video_rgb.0, video_rgb.1, video_rgb.2, 255),
    }
}

pub mod chroma;
pub mod difference;

pub use chroma::Chroma;
pub use difference::Difference;
