use core::{
    fmt,
    ops::{Add, Sub},
    time::Duration,
};

const NANOS_PER_SEC: u64 = 1_000_000_000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(Duration);

impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl Instant {
    pub const ZERO: Self = Self(Duration::ZERO);
    pub const MAX: Self = Self(Duration::MAX);

    pub fn now() -> Self {
        super::now()
    }

    pub fn elapsed(&self) -> Duration {
        Self::now().duration_since(*self)
    }

    pub fn duration_since_epoc(&self) -> Duration {
        self.duration_since(Self::ZERO)
    }

    pub fn duration_since(&self, earlier: Self) -> Duration {
        self.0 - earlier.0
    }

    pub fn from_timer_ticks(timer_ticks: u64, timer_frequency: u64) -> Self {
        let sec = timer_ticks / timer_frequency;
        let subsec = timer_ticks % timer_frequency;
        let subsec_nanos = (subsec * NANOS_PER_SEC / timer_frequency)
            .try_into()
            .unwrap();
        Self(Duration::new(sec, subsec_nanos))
    }

    pub fn as_timer_ticks(&self, timer_frequency: u64) -> u64 {
        let sec = self.0.as_secs();
        let subsec_nanos = self.0.subsec_nanos();
        sec * timer_frequency + u64::from(subsec_nanos) * timer_frequency / NANOS_PER_SEC
    }
}

impl Add<Duration> for Instant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<Self> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.duration_since(rhs)
    }
}
