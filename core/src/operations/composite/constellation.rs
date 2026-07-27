 fn constellation_position(&self, time: f64, group: u32, count: u32, distance: f64) -> (f64, f64) {
        let (hx, hy) = self.hub(time);

        let angle = (PI * 2.0 / count as f64) * group as f64 + time * 0.15;

        (hx + angle.cos() * distance, hy + angle.sin() * distance)
    }