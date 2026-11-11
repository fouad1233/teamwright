//! The clock of the computer.

use chrono::{DurationRound, TimeDelta, Utc};
use teamwright_domain::Timestamp;

use crate::ports::Clock;

/// Reads the time from the operating system.
///
/// Times are cut to whole milliseconds. Storage keeps milliseconds, so a
/// value that is stored and read back equals the value that was handed out.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let now = Utc::now();
        // Cutting can only fail for dates outside the range the calendar
        // library supports, in which case the uncut time is the best answer.
        now.duration_trunc(TimeDelta::milliseconds(1))
            .unwrap_or(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_time_has_no_fraction_below_a_millisecond() {
        let now = SystemClock.now();

        assert_eq!(now.timestamp_subsec_nanos() % 1_000_000, 0);
    }
}
