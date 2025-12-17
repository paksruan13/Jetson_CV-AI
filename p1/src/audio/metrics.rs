#[derive(Debug, Clone, Copy)]
pub struct AudioMetrics {
    pub rms: f32,
    pub peak: f32,
    pub db: f32,
}

impl AudioMetrics {
    pub fn new() -> Self {
        Self {
            rms: 0.0,
            peak: 0.0,
            db: -60.0,
        }
    }

    pub fn update(&mut self, rms: f32, peak: f32, db: f32) {
        self.rms = rms;
        self.peak = peak;
        self.db = db;
    }
}

impl Default for AudioMetrics {
    fn default() -> Self {
        Self::new()
    }
}