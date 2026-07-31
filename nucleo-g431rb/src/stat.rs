use defmt::Format;

/// Welford's online algorithm for mean and standard deviation.
pub struct Stats {
    count: usize,
    mean: f32,
    m2: f32,
    min: f32,
    max: f32,
}

impl Format for Stats {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "n={} mean={} std={} min={} max={}",
            self.count,
            self.mean,
            self.std_dev(),
            self.min,
            self.max,
        );
    }
}

impl Stats {
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: f32::MAX,
            max: f32::MIN,
        }
    }

    pub fn update(&mut self, value: f32) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f32;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn mean(&self) -> f32 {
        self.mean
    }

    pub fn min(&self) -> f32 {
        self.min
    }

    pub fn max(&self) -> f32 {
        self.max
    }

    pub fn std_dev(&self) -> f32 {
        if self.count < 2 {
            return 0.0;
        }
        libm::sqrtf(self.m2 / (self.count - 1) as f32)
    }
}
