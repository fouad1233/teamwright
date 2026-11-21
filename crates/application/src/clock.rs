//! The clock of the computer.

use chrono::{DurationRound, FixedOffset, Local, NaiveDateTime, Offset, TimeDelta, TimeZone, Utc};
use teamwright_domain::{CalendarDate, Timestamp};

use crate::ports::{Clock, utc_offset_of};

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

    /// The day in the time zone the operating system is set to, where the
    /// manager is.
    fn day_of(&self, moment: Timestamp) -> CalendarDate {
        moment.with_timezone(&Local).date_naive()
    }

    fn local_time(&self, moment: Timestamp) -> NaiveDateTime {
        moment.with_timezone(&Local).naive_local()
    }

    /// The zone of this computer is known by its offsets. Another zone is
    /// known only if it is this computer's under its name, or Coordinated
    /// Universal Time: the program carries no database of the zones of the
    /// world. Claude Code runs on this computer and names its zone, so the
    /// zone it names is nearly always this one.
    fn offset_in(&self, zone: Option<&str>, local: NaiveDateTime) -> Option<FixedOffset> {
        let is_here = match zone {
            None => true,
            Some(name) => iana_time_zone::get_timezone().is_ok_and(|here| here == name),
        };
        if is_here {
            return Local
                .from_local_datetime(&local)
                .earliest()
                .map(|moment| moment.offset().fix());
        }
        zone.and_then(utc_offset_of)
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
