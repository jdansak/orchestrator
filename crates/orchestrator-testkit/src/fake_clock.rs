//! A controllable clock.
//!
//! Rollout timing, health-check intervals, and session expiry are all time-dependent. Tests
//! that sleep for real minutes are slow and flaky. The `FakeClock` lets a test advance time
//! instantly.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A clock whose current time can be advanced on demand.
#[derive(Debug, Clone)]
pub struct FakeClock {
    /// The current time, in nanoseconds since the epoch.
    now: Arc<AtomicU64>,
}

impl FakeClock {
    /// Creates a clock starting at the given offset from the Unix epoch.
    pub fn new() -> Self {
        Self::at(Duration::ZERO)
    }

    /// Creates a clock starting at the given offset from the Unix epoch.
    pub fn at(offset: Duration) -> Self {
        Self {
            now: Arc::new(AtomicU64::new(offset.as_nanos() as u64)),
        }
    }

    /// The current time.
    pub fn now(&self) -> Duration {
        Duration::from_nanos(self.now.load(Ordering::SeqCst))
    }

    /// Advances the clock.
    pub fn advance(&self, duration: Duration) {
        self.now
            .fetch_add(duration.as_nanos() as u64, Ordering::SeqCst);
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_the_current_time() {
        let clock = FakeClock::new();
        assert_eq!(clock.now(), Duration::ZERO);
        clock.advance(Duration::from_secs(61));
        assert_eq!(clock.now(), Duration::from_secs(61));
    }

    #[test]
    fn clones_share_the_same_time() {
        let clock = FakeClock::new();
        let clone = clock.clone();
        clock.advance(Duration::from_secs(5));
        assert_eq!(clone.now(), Duration::from_secs(5));
    }
}
