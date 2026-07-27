    fn hub(&self, time: f64) -> (f64, f64) {
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;

        (
            w / 2.0 + (time * 0.4 + self.hub_phase).sin() * 40.0,
            h / 2.0 + (time * 0.32 + self.hub_phase).cos() * 30.0,
        )
    }