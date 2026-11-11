//! Time, as the domain sees it.
//!
//! The domain never asks what time it is. Every operation that needs the
//! current moment receives it as an argument, which keeps the rules
//! deterministic and easy to test.

/// A moment in time, in Coordinated Universal Time.
pub type Timestamp = chrono::DateTime<chrono::Utc>;

/// A day on the calendar, without a time of day or a time zone.
///
/// Start dates and due dates are calendar dates: a task is due "on Friday",
/// wherever the manager happens to be.
pub type CalendarDate = chrono::NaiveDate;
