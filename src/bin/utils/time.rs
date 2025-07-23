use core::ops::{Add, AddAssign, Sub, SubAssign};

/// Time duration with microsecond resolution
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Duration(u32);

impl Duration {
    /// Create a time duration in seconds
    #[allow(unused)]
    pub fn from_secs(secs: u32) -> Duration {
        Duration(secs * 1000_000)
    }

    /// Create a time duration in milliseconds
    #[allow(unused)]
    pub fn from_msecs(msecs: u32) -> Duration {
        Duration(msecs * 1000)
    }

    /// Create a time duration in microseconds
    #[allow(unused)]
    pub fn from_usecs(usecs: u32) -> Duration {
        Duration(usecs)
    }

    /// Return duration in microseconds
    #[allow(unused)]
    pub fn usecs(self) -> u32 {
        self.0
    }
}

impl Add<Duration> for Duration {
    type Output = Duration;
    fn add(self, rhs: Duration) -> Self::Output {
        Duration(self.0 + rhs.0)
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Sub<Duration> for Duration {
    type Output = Duration;
    fn sub(self, rhs: Duration) -> Self::Output {
        Duration(self.0 - rhs.0)
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
    }
}

/// Time with microsecond resolution
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Instant(u32);

impl Instant {

    /// Create a time in seconds
    #[allow(unused)]
    pub fn from_secs(secs: u32) -> Instant {
        Instant(secs * 1000_000)
    }

    /// Create a time in milliseconds
    #[allow(unused)]
    pub fn from_msecs(msecs: u32) -> Instant {
        Instant(msecs * 1000)
    }

    /// Create a time in microseconds
    #[allow(unused)]
    pub fn from_usecs(usecs: u32) -> Instant {
        Instant(usecs)
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Self::Output {
        Instant(self.0.wrapping_add(rhs.0))
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, rhs: Duration) -> Self::Output {
        Instant(self.0.wrapping_sub(rhs.0))
    }
}
