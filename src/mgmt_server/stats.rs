#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    size: f32,
    mean: Option<f32>,
    square_deviation: f32,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            size: 0f32,
            mean: None,
            square_deviation: 0f32,
        }
    }
}

impl Stats {
    pub fn add(&mut self, value: f32) {
        self.size += 1.0;
        self.mean = if let Some(mean) = self.mean {
            let delta = value - mean;
            let new_mean = mean + delta / self.size;
            self.square_deviation += delta * (value - new_mean);
            Some(new_mean)
        } else {
            Some(value)
        }
    }
}
