//! Converts between the values of the domain and the text the database
//! keeps.
//!
//! The forms are fixed by the schema: moments are UTC in RFC 3339 with
//! milliseconds, calendar dates are `YYYY-MM-DD`, and identifiers and
//! enumerations use the text forms the domain defines.

use std::str::FromStr;

use chrono::{DateTime, SecondsFormat, Utc};
use teamwright_domain::{CalendarDate, DomainError, Timestamp};

use crate::error::DatabaseError;

/// The form calendar dates are stored in.
const DATE_FORMAT: &str = "%Y-%m-%d";

/// Writes a moment as text.
pub(crate) fn moment_to_text(moment: Timestamp) -> String {
    moment.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Reads a moment from text.
pub(crate) fn moment_from_text(text: &str) -> Result<Timestamp, DatabaseError> {
    DateTime::parse_from_rfc3339(text)
        .map(|moment| moment.with_timezone(&Utc))
        .map_err(|reason| DatabaseError::damaged("the moment", text, reason))
}

/// Writes a calendar date as text.
pub(crate) fn date_to_text(date: CalendarDate) -> String {
    date.format(DATE_FORMAT).to_string()
}

/// Reads a calendar date from text.
pub(crate) fn date_from_text(text: &str) -> Result<CalendarDate, DatabaseError> {
    CalendarDate::parse_from_str(text, DATE_FORMAT)
        .map_err(|reason| DatabaseError::damaged("the date", text, reason))
}

/// Reads a domain value that has a text form, such as an identifier or a
/// priority.
pub(crate) fn parse<T>(text: &str) -> Result<T, DatabaseError>
where
    T: FromStr<Err = DomainError>,
{
    text.parse()
        .map_err(|reason| DatabaseError::damaged("the value", text, reason))
}

/// Turns a rule broken by stored data into a storage error that says which
/// record is affected.
pub(crate) fn damaged_record(kind: &str, id: &str, reason: &DomainError) -> DatabaseError {
    DatabaseError::Damaged(format!("the {kind} {id} breaks a rule: {reason}"))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use teamwright_domain::{Priority, TaskId};

    use super::*;

    #[test]
    fn a_moment_is_written_in_utc_with_milliseconds() {
        let moment = Utc.with_ymd_and_hms(2026, 11, 11, 9, 5, 7).unwrap()
            + chrono::TimeDelta::milliseconds(42);

        assert_eq!(moment_to_text(moment), "2026-11-11T09:05:07.042Z");
    }

    #[test]
    fn a_moment_survives_being_written_and_read_back() {
        let moment = Utc.with_ymd_and_hms(2026, 11, 11, 9, 5, 7).unwrap();

        assert_eq!(moment_from_text(&moment_to_text(moment)).unwrap(), moment);
    }

    #[test]
    fn text_that_is_not_a_moment_is_reported_as_damaged() {
        let error = moment_from_text("yesterday").unwrap_err();

        assert!(matches!(error, DatabaseError::Damaged(message) if message.contains("yesterday")));
    }

    #[test]
    fn a_date_is_written_as_year_month_day() {
        let date = CalendarDate::from_ymd_opt(2026, 3, 9).unwrap();

        assert_eq!(date_to_text(date), "2026-03-09");
        assert_eq!(date_from_text("2026-03-09").unwrap(), date);
    }

    #[test]
    fn identifiers_and_enumerations_are_read_through_their_text_form() {
        let id = TaskId::generate();

        assert_eq!(parse::<TaskId>(&id.to_string()).unwrap(), id);
        assert_eq!(parse::<Priority>("urgent").unwrap(), Priority::Urgent);
        assert!(parse::<Priority>("whenever").is_err());
    }
}
