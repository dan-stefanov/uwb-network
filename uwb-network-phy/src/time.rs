use core::ops;

/// Unsigned duration
///
/// The unit is set to HRP Ranging counter time unit (RCTU), which is
/// 1/128 of the 499.2 MHz chipping period
/// Max value is ~9 years
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Duration(u64);

impl Duration {
    pub const MAX: Duration = Duration::from_ticks(u64::MAX);
    pub const ZERO: Duration = Duration::from_ticks(0);

    /// Ranging Counter Time Unit, ~15.65 ps
    ///
    /// Defined at IEEE 802.15.4z section 6.9.1.4
    pub const RCTU: Duration = Duration::from_ticks(1);

    /// 499.2 MHz chip duration, ~2.003 ns
    ///
    /// Defined at IEEE 802.15.4 section 15.2.4
    pub const CHIP: Duration = Self::RCTU.mul_u32(128);

    /// Ranging Scheduling Time Unit, ~0.833 us
    ///
    /// Defined at IEEE 802.15.4z section 6.9.1.5
    pub const RSTU: Duration = Self::CHIP.mul_u32(416);

    pub const SECOND: Duration = Self::CHIP.mul_u32(499_200_000);

    pub const fn from_ticks(ticks: u64) -> Self {
        Self(ticks)
    }

    pub const fn as_ticks(self) -> u64 {
        self.0
    }

    pub const fn min(self, other: Duration) -> Duration {
        if self.0 <= other.0 { self } else { other }
    }

    pub const fn max(self, other: Duration) -> Duration {
        if self.0 >= other.0 { self } else { other }
    }

    /// Adds one Duration to another, returning a new Duration or None in the event of an overflow.
    pub const fn checked_add(self, rhs: Duration) -> Option<Duration> {
        match self.0.checked_add(rhs.0) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Subtracts one Duration to another, returning a new Duration or None in the event of an overflow.
    pub const fn checked_sub(self, rhs: Duration) -> Option<Duration> {
        match self.0.checked_sub(rhs.0) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Divides one Duration by another, returning a factor or None if the later is zero.
    pub const fn checked_div(self, rhs: Duration) -> Option<u64> {
        match self.0.checked_div(rhs.0) {
            Some(value) => Some(value),
            None => None,
        }
    }

    /// Divides one Duration by another, rounding up and returning a factor or None if the later is zero.
    pub const fn checked_div_ceil(self, rhs: Duration) -> Option<u64> {
        if rhs.0 == 0 {
            None
        } else {
            Some(self.0.div_ceil(rhs.0))
        }
    }

    /// Divides one Duration by another, returning a reminder or None if the later is zero.
    pub const fn checked_rem(self, rhs: Duration) -> Option<Duration> {
        match self.0.checked_rem(rhs.0) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Multiplies one Duration by a scalar u32, returning a new Duration or None in the event of an overflow.
    pub const fn checked_mul_u32(self, rhs: u32) -> Option<Duration> {
        match self.0.checked_mul(rhs as u64) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Divides one Duration a scalar u32, returning a new Duration or None in the event of an overflow.
    pub const fn checked_div_u32(self, rhs: u32) -> Option<Duration> {
        match self.0.checked_div(rhs as u64) {
            Some(ticks) => Some(Self(ticks)),
            None => None,
        }
    }

    /// Adds one Duration to another
    pub const fn add(self, rhs: Duration) -> Duration {
        match self.checked_add(rhs) {
            Some(duration) => duration,
            None => core::panic!("overflow when adding durations"),
        }
    }

    /// Subtracts one Duration to another
    pub const fn sub(self, rhs: Duration) -> Duration {
        match self.checked_sub(rhs) {
            Some(duration) => duration,
            None => core::panic!("overflow when subtracting durations"),
        }
    }

    /// Divides one Duration by another
    pub const fn div(self, rhs: Duration) -> u64 {
        match self.checked_div(rhs) {
            Some(duration) => duration,
            None => core::panic!("divide by zero error when dividing duration by another"),
        }
    }

    /// Divides one Duration by another, rounding up
    pub const fn div_ceil(self, rhs: Duration) -> u64 {
        match self.checked_div_ceil(rhs) {
            Some(duration) => duration,
            None => core::panic!("divide by zero error when dividing duration by another"),
        }
    }

    /// Reminder after division of one Duration by another
    pub const fn rem(self, rhs: Duration) -> Duration {
        match self.checked_rem(rhs) {
            Some(duration) => duration,
            None => core::panic!("divide by zero error when dividing duration by another"),
        }
    }

    /// Multiplies one Duration by a scalar u32
    pub const fn mul_u32(self, rhs: u32) -> Duration {
        match self.checked_mul_u32(rhs) {
            Some(duration) => duration,
            None => core::panic!("overflow when multiplying duration by scalar"),
        }
    }

    /// Divides one Duration a scalar u32
    pub const fn div_u32(self, rhs: u32) -> Duration {
        match self.checked_div_u32(rhs) {
            Some(duration) => duration,
            None => core::panic!("divide by zero error when dividing duration by scalar"),
        }
    }
}

impl ops::Add for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Duration {
        self.checked_add(rhs)
            .expect("overflow when adding durations")
    }
}

impl ops::AddAssign for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl ops::Sub for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Duration {
        self.checked_sub(rhs)
            .expect("overflow when subtracting durations")
    }
}

impl ops::SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl ops::Mul<u32> for Duration {
    type Output = Duration;

    fn mul(self, rhs: u32) -> Duration {
        self.checked_mul_u32(rhs)
            .expect("overflow when multiplying duration by scalar")
    }
}

impl ops::Mul<Duration> for u32 {
    type Output = Duration;

    fn mul(self, rhs: Duration) -> Duration {
        rhs * self
    }
}

impl ops::MulAssign<u32> for Duration {
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs;
    }
}

impl ops::Div<u32> for Duration {
    type Output = Duration;

    fn div(self, rhs: u32) -> Duration {
        self.checked_div_u32(rhs)
            .expect("divide by zero error when dividing duration by scalar")
    }
}

impl ops::DivAssign<u32> for Duration {
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs;
    }
}

pub trait CyclicTimebase: Copy + Eq + core::fmt::Debug {
    const PERIOD: Duration;
}

/// Represent a periodic time instant
///
/// The unit is set to HRP Ranging counter time unit (RCTU), ~15.65 ps
/// Value is given modulo device-specific timestamp period
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Instant<T> {
    ticks: u64,
    timebase: core::marker::PhantomData<T>,
}

#[cfg(feature = "defmt")]
impl<T> defmt::Format for Instant<T> {
    fn format(&self, formatter: defmt::Formatter) {
        defmt::write!(formatter, "Instant({})", self.ticks);
    }
}

impl<T> core::fmt::Debug for Instant<T> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.debug_tuple("Instant").field(&self.ticks).finish()
    }
}

impl<T> Instant<T> {
    const fn from_ticks_unchecked(ticks: u64) -> Self {
        Self {
            ticks,
            timebase: core::marker::PhantomData,
        }
    }
}

impl<T: CyclicTimebase> Instant<T> {
    const _ASSERT_NON_ZERO: u64 = T::PERIOD.as_ticks() - 1;

    pub const fn period(self) -> Duration {
        T::PERIOD
    }

    pub fn try_from_ticks(ticks: u64) -> Option<Self> {
        if ticks < T::PERIOD.as_ticks() {
            Some(Self::from_ticks_unchecked(ticks))
        } else {
            None
        }
    }

    pub const fn as_ticks(self) -> u64 {
        self.ticks
    }

    pub const fn add(self, duration: Duration) -> Self {
        let period = T::PERIOD.as_ticks();
        let a = self.as_ticks();
        let b = duration.as_ticks() % period;

        let ticks = if a >= period - b {
            a - (period - b)
        } else {
            a + b
        };
        Self::from_ticks_unchecked(ticks)
    }

    pub const fn sub_duration(self, duration: Duration) -> Self {
        let period = T::PERIOD.as_ticks();
        let a = self.as_ticks();
        let b = duration.as_ticks() % period;

        let ticks = if a < b { a + (period - b) } else { a - b };
        Self::from_ticks_unchecked(ticks)
    }

    pub const fn sub_instant(self, rhs: Instant<T>) -> Duration {
        let period = T::PERIOD.as_ticks();
        let a = self.as_ticks();
        let b = rhs.as_ticks();

        let ticks = if a < b { a + (period - b) } else { a - b };
        Duration::from_ticks(ticks)
    }
}

impl<T: CyclicTimebase> ops::Add<Duration> for Instant<T> {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        self.add(rhs)
    }
}

impl<T: CyclicTimebase> ops::AddAssign<Duration> for Instant<T> {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl<T: CyclicTimebase> ops::Sub<Duration> for Instant<T> {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self {
        self.sub_duration(rhs)
    }
}

impl<T: CyclicTimebase> ops::SubAssign<Duration> for Instant<T> {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl<T: CyclicTimebase> ops::Sub<Instant<T>> for Instant<T> {
    type Output = Duration;

    fn sub(self, rhs: Instant<T>) -> Duration {
        self.sub_instant(rhs)
    }
}
