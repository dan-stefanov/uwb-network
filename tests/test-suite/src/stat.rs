use bnum::{
    cast::{As, CastFrom},
    types::{I256, I512},
};

const I256_ZERO: I256 = I256::from_bytes([0; 32]);
const I512_ZERO: I512 = I512::from_bytes([0; 64]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaxSamplesReached;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DegenerateProblem;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Report {
    pub sample_count: u32,
    pub intercept: f32,
    pub slope: f32,
    pub mse: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LinearRegression {
    count: u32,
    sum_x: i128,
    sum_y: i128,
    sum_xx: i128,
    sum_xy: i128,
    sum_yy: I256,
}

impl LinearRegression {
    pub const fn new() -> Self {
        Self {
            count: 0,
            sum_x: 0,
            sum_y: 0,
            sum_xx: 0,
            sum_xy: 0,
            sum_yy: I256_ZERO,
        }
    }

    pub fn add(&mut self, x: i32, y: i64) -> Result<(), MaxSamplesReached> {
        self.count = self.count.checked_add(1).ok_or(MaxSamplesReached)?;

        // At most 32 bits.
        let x = i128::from(x);
        // At most 64 bits.
        let y = i128::from(y);
        // At most 63 bits.
        self.sum_x += x;
        // At most 95 bits.
        self.sum_y += y;
        // At most 94 bits.
        self.sum_xx += x * x;
        // At most 126 bits.
        self.sum_xy += x * y;
        // At most 158 bits.
        self.sum_yy += I256::cast_from(y * y);

        Ok(())
    }

    pub fn fit(&self) -> Result<Report, DegenerateProblem> {
        // At most 32 bits.
        let count = I512::cast_from(self.count);
        // At most 63 bits.
        let sum_x = I512::cast_from(self.sum_x);
        // At most 95 bits.
        let sum_y = I512::cast_from(self.sum_y);
        // At most 94 bits.
        let sum_xx = I512::cast_from(self.sum_xx);
        // At most 126 bits.
        let sum_xy = I512::cast_from(self.sum_xy);
        // At most 158 bits.
        let sum_yy = I512::cast_from(self.sum_yy);

        // At most 126 bits.
        let d = count * sum_xx - sum_x * sum_x;
        // At most 190 bits.
        let a = sum_y * sum_xx - sum_x * sum_xy;
        // At most 158 bits.
        let b = count * sum_xy - sum_x * sum_y;
        // Final value: at most 284 bits. Its `a * sum_y` term reaches 285 bits.
        let e = d * sum_yy - a * sum_y - b * sum_xy;

        if d == I512_ZERO {
            return Err(DegenerateProblem);
        }

        let intercept = a.as_::<f32>() / d.as_::<f32>();
        let slope = b.as_::<f32>() / d.as_::<f32>();
        let mse = e.as_::<f32>() / (count.as_::<f32>() * d.as_::<f32>());

        Ok(Report {
            sample_count: self.count,
            intercept,
            slope,
            mse,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DegenerateProblem, LinearRegression};

    #[test]
    fn fits_an_exact_line() {
        let mut regression = LinearRegression::new();
        for x in 0..10 {
            regression.add(x, 3 * i64::from(x) + 7).unwrap();
        }

        let fit = regression.fit().unwrap();
        assert_eq!(fit.sample_count, 10);
        assert_eq!(fit.slope, 3.0);
        assert_eq!(fit.intercept, 7.0);
        assert_eq!(fit.mse, 0.0);
    }

    #[test]
    fn retains_precision_for_large_products() {
        let mut regression = LinearRegression::new();
        let offset = i64::MAX - i64::from(i32::MAX);
        for x in [i32::MAX - 2, i32::MAX - 1, i32::MAX] {
            regression.add(x, offset + i64::from(x)).unwrap();
        }

        let fit = regression.fit().unwrap();
        assert_eq!(fit.slope, 1.0);
        assert_eq!(fit.intercept, offset as f32);
    }

    #[test]
    fn fits_a_line_with_negative_samples() {
        let mut regression = LinearRegression::new();
        for x in 0..10 {
            regression.add(x, -3 * i64::from(x) - 7).unwrap();
        }

        let fit = regression.fit().unwrap();
        assert_eq!(fit.slope, -3.0);
        assert_eq!(fit.intercept, -7.0);
        assert_eq!(fit.mse, 0.0);
    }

    #[test]
    fn calculates_mse() {
        let mut regression = LinearRegression::new();
        for (x, y) in [(0, 0), (1, 2), (2, 1)] {
            regression.add(x, y).unwrap();
        }

        let fit = regression.fit().unwrap();
        assert_eq!(fit.slope, 0.5);
        assert_eq!(fit.intercept, 0.5);
        assert_eq!(fit.mse, 0.5);
    }

    #[test]
    fn rejects_a_vertical_fit() {
        let mut regression = LinearRegression::new();
        regression.add(4, 1).unwrap();
        regression.add(4, 2).unwrap();

        assert_eq!(regression.fit(), Err(DegenerateProblem));
    }
}
