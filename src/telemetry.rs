#[derive(Debug)]
pub struct TelemetryStats {
    pub sum_db: f32,
    pub max_db: f32,
    pub count: u32,
}

impl TelemetryStats {
    pub fn new() -> Self {
        Self {
            sum_db: 0.0,
            max_db: f32::MIN,
            count: 0,
        }
    }

    pub fn add(&mut self, db: f32) {
        self.sum_db += db;
        self.count += 1;
        if db > self.max_db {
            self.max_db = db;
        }
    }

    pub fn current_avg(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.sum_db / self.count as f32
        }
    }

    pub fn take_and_reset(&mut self) -> Option<(f32, f32)> {
        if self.count == 0 {
            return None;
        }
        let avg = self.sum_db / self.count as f32;
        let max = self.max_db;

        self.sum_db = 0.0;
        self.count = 0;
        self.max_db = f32::MIN;

        Some((avg, max))
    }
}
