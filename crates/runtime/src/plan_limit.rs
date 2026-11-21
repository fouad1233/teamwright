//! Reading, from the words of Claude Code, that the limit of the user's plan
//! was reached, and when it resets.
//!
//! The program says it in a sentence for people, and its wording changes
//! with its versions. Seen and read in the program:
//!
//! - "You've hit your 5-hour limit · resets 10:40pm (Europe/Istanbul)"
//!   (a turn of 21 November 2026, a newer version than 2.1.289)
//! - "You've hit your session limit · resets 10pm (Europe/Istanbul)",
//!   "You've hit your weekly limit · resets Nov 23, 10am (Europe/Istanbul)",
//!   "You've hit your Opus limit · ..." and "You've reached your Fable
//!   limit" (the texts of version 2.1.289)
//! - "Claude AI usage limit reached|1794470400" (older versions, with the
//!   moment of the reset as a number)
//!
//! So the reader is tolerant: an opening that says the user hit or reached
//! one of their limits, a name of the limit, and perhaps "resets" with a
//! time. It never takes an ordinary error for the limit: "Rate limit
//! reached" of an API key, a server that is overloaded, or any sentence
//! that does not open this way, is not the plan's limit.

use chrono::{NaiveTime, TimeZone, Utc};
use teamwright_application::{PlanLimitHit, PlanReset, PlanWindowKind, SaidDate, SaidReset};

/// The openings that say a limit of the user's plan was reached, in lower
/// case and with a straight apostrophe.
const OPENINGS: &[&str] = &[
    "you've hit your ",
    "you have hit your ",
    "you've reached your ",
    "you have reached your ",
];

/// How older versions said it, followed by `|` and the moment of the reset
/// in seconds.
const OLD_SENTENCE: &str = "usage limit reached";

/// The limit of the plan, if a text says it was reached: the first line of
/// the text that says so.
#[must_use]
pub fn limit_in_words(text: &str) -> Option<PlanLimitHit> {
    text.lines().find_map(limit_in_line)
}

fn limit_in_line(line: &str) -> Option<PlanLimitHit> {
    let line = line.replace(['\u{2019}', '\u{2018}'], "'");
    let lower = line.to_lowercase();
    if let Some(found) = lower.find(OLD_SENTENCE) {
        // Only at the start of the line, after an optional "Claude AI".
        let before = lower[..found].trim();
        if !(before.is_empty() || before == "claude ai" || before == "claude") {
            return None;
        }
        let rest = &line[found + OLD_SENTENCE.len()..];
        let resets = rest
            .strip_prefix('|')
            .and_then(|seconds| seconds.trim().parse::<i64>().ok())
            .and_then(|seconds| Utc.timestamp_opt(seconds, 0).single())
            .map_or(PlanReset::Unknown, PlanReset::At);
        return Some(PlanLimitHit {
            window: None,
            resets,
        });
    }

    let trimmed = lower.trim_start();
    let opening = OPENINGS
        .iter()
        .find(|opening| trimmed.starts_with(**opening))?;
    let start = lower.len() - trimmed.len() + opening.len();
    // The name of the limit runs to the word "limit", within the first
    // part of the sentence.
    let first_part_end = part_end(&lower, start);
    let name_part = &lower[start..first_part_end];
    let limit_at = name_part.find("limit")?;
    let name = name_part[..limit_at].trim();
    if name.split_whitespace().count() > 3 {
        return None;
    }
    // The zone keeps its capitals from the line, when lowering the case
    // left the places of the characters as they were.
    let original = if line.len() == lower.len() {
        line.as_str()
    } else {
        lower.as_str()
    };
    let resets = lower[start..]
        .find("resets ")
        .map(|at| start + at + "resets ".len())
        .and_then(|at| original.get(at..part_end(original, at)))
        .and_then(reset_in_words)
        .map_or(PlanReset::Unknown, PlanReset::Said);
    Some(PlanLimitHit {
        window: window_named(name),
        resets,
    })
}

/// Where the part of a sentence that starts at `from` ends: at the next
/// middle dot, or at the end of the line.
fn part_end(line: &str, from: usize) -> usize {
    line.get(from..)
        .and_then(|rest| rest.find('\u{b7}'))
        .map_or(line.len(), |found| from + found)
}

/// Which window a limit is, by the name the program gave it.
fn window_named(name: &str) -> Option<PlanWindowKind> {
    let name = name.trim();
    let has = |word: &str| name.contains(word);
    if has("5-hour") || has("five-hour") || has("5 hour") || has("five hour") || has("session") {
        Some(PlanWindowKind::FiveHours)
    } else if has("weekly") || has("7-day") || has("seven-day") || has("week") {
        Some(PlanWindowKind::Week)
    } else if name.is_empty() || name == "usage" || name == "plan" || name == "plan's" {
        None
    } else {
        Some(PlanWindowKind::Other(name.to_owned()))
    }
}

/// A reset time in words: "10:40pm (Europe/Istanbul)", "10pm", "22:40",
/// "Nov 23, 10am (Europe/Istanbul)" or "Nov 23, 2027, 10:40am". Nothing if
/// it is not understood, for example "in 3h", which is not a time.
#[must_use]
pub fn reset_in_words(words: &str) -> Option<SaidReset> {
    let mut words = words.trim().trim_end_matches(['.', ',']).trim();
    let mut zone = None;
    if let (Some(open), true) = (words.rfind('('), words.ends_with(')')) {
        let name = words[open + 1..words.len() - 1].trim();
        if !name.is_empty() {
            zone = Some(name.to_owned());
        }
        words = words[..open].trim();
    }
    let words = words
        .strip_prefix("at ")
        .or_else(|| words.strip_prefix("on "))
        .unwrap_or(words);
    let parts: Vec<&str> = words.split(',').map(str::trim).collect();
    let (time_part, date_parts) = parts.split_last()?;
    let time = time_in_words(time_part)?;
    let date = match date_parts {
        [] => None,
        [month_and_day] => Some(date_in_words(month_and_day, None)?),
        [month_and_day, year] => Some(date_in_words(month_and_day, Some(year.parse().ok()?))?),
        _ => return None,
    };
    Some(SaidReset { time, date, zone })
}

/// A time of day: "10:40pm", "10pm", "10 PM", "10:40 a.m." or "22:40".
fn time_in_words(words: &str) -> Option<NaiveTime> {
    let compact: String = words
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '.')
        .collect::<String>()
        .to_lowercase();
    let (clock, half) = if let Some(clock) = compact.strip_suffix("am") {
        (clock, Some(false))
    } else if let Some(clock) = compact.strip_suffix("pm") {
        (clock, Some(true))
    } else {
        (compact.as_str(), None)
    };
    let (hour, minute) = match clock.split_once(':') {
        Some((hour, minute)) if minute.len() == 2 => (hour, minute.parse::<u32>().ok()?),
        Some(_) => return None,
        None if half.is_some() => (clock, 0),
        None => return None,
    };
    if hour.is_empty() || hour.len() > 2 || !hour.chars().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    let hour: u32 = hour.parse().ok()?;
    let hour = match half {
        None if hour < 24 => hour,
        Some(afternoon) if (1..=12).contains(&hour) => match (afternoon, hour) {
            (false, 12) => 0,
            (true, 12) => 12,
            (false, hour) => hour,
            (true, hour) => hour + 12,
        },
        _ => return None,
    };
    NaiveTime::from_hms_opt(hour, minute, 0)
}

/// A date: "Nov 23", with the year if it was given.
fn date_in_words(words: &str, year: Option<i32>) -> Option<SaidDate> {
    let (month, day) = words.split_once(' ')?;
    let month = month_named(month)?;
    let day: u32 = day.trim().parse().ok()?;
    (1..=31)
        .contains(&day)
        .then_some(SaidDate { month, day, year })
}

fn month_named(name: &str) -> Option<u32> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let name = name.trim().trim_end_matches('.').to_lowercase();
    let short = name.get(..3)?;
    MONTHS
        .iter()
        .position(|month| *month == short)
        .and_then(|place| u32::try_from(place + 1).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
    }

    fn said(words: &str) -> SaidReset {
        let hit = limit_in_words(words).unwrap_or_else(|| panic!("not read: {words}"));
        match hit.resets {
            PlanReset::Said(said) => said,
            other => panic!("{words}: {other:?}"),
        }
    }

    #[test]
    fn the_sentence_of_tonight_is_read_with_its_window_time_and_zone() {
        let hit =
            limit_in_words("You've hit your 5-hour limit \u{b7} resets 10:40pm (Europe/Istanbul)")
                .unwrap();
        assert_eq!(hit.window, Some(PlanWindowKind::FiveHours));
        assert_eq!(
            hit.resets,
            PlanReset::Said(SaidReset {
                time: time(22, 40),
                date: None,
                zone: Some("Europe/Istanbul".to_owned()),
            })
        );
    }

    #[test]
    fn other_wordings_of_the_limit_are_read() {
        let cases = [
            (
                "You've hit your session limit \u{b7} resets 10pm (Europe/Istanbul)",
                Some(PlanWindowKind::FiveHours),
            ),
            (
                "You\u{2019}ve hit your weekly limit \u{b7} resets Nov 23, 10am (Europe/Istanbul)",
                Some(PlanWindowKind::Week),
            ),
            (
                "You have reached your five-hour limit. Resets 9:05 AM",
                Some(PlanWindowKind::FiveHours),
            ),
            (
                "You've hit your Opus limit \u{b7} resets 3am (America/New_York)",
                Some(PlanWindowKind::Other("opus".to_owned())),
            ),
            (
                "You've reached your Fable limit.",
                Some(PlanWindowKind::Other("fable".to_owned())),
            ),
            ("You've hit your usage limit", None),
            (
                "You've hit your 5-hour limit \u{b7} resets 10:40pm (Europe/Istanbul) \u{b7} progress saved",
                Some(PlanWindowKind::FiveHours),
            ),
        ];
        for (words, window) in cases {
            let hit = limit_in_words(words).unwrap_or_else(|| panic!("not read: {words}"));
            assert_eq!(hit.window, window, "{words}");
        }
    }

    #[test]
    fn the_sentence_may_stand_on_any_line() {
        let hit = limit_in_words("API Error: 429\nYou've hit your weekly limit").unwrap();
        assert_eq!(hit.window, Some(PlanWindowKind::Week));
        assert_eq!(hit.resets, PlanReset::Unknown);
    }

    #[test]
    fn the_old_sentence_carries_the_moment_as_a_number() {
        let hit = limit_in_words("Claude AI usage limit reached|1794470400").unwrap();
        assert_eq!(
            hit.resets,
            PlanReset::At(Utc.timestamp_opt(1_794_470_400, 0).unwrap())
        );
        assert_eq!(
            limit_in_words("Claude AI usage limit reached")
                .unwrap()
                .resets,
            PlanReset::Unknown
        );
    }

    #[test]
    fn ordinary_errors_are_not_the_limit_of_the_plan() {
        for words in [
            "API Error: Rate limit reached for requests",
            "API Error: 429 {\"type\":\"error\",\"error\":{\"type\":\"rate_limit_error\"}}",
            "Overloaded",
            "Not logged in \u{b7} Please run /login",
            "The request hit your limit of patience",
            "I have hit your limit of five files, as you asked.",
            "You've hit your team's shared budget. Switch to another model to continue.",
            "You're out of usage credits",
            "The output reached the usage limit reached by the tool",
            "",
        ] {
            assert_eq!(limit_in_words(words), None, "{words}");
        }
    }

    #[test]
    fn reset_times_are_read_in_their_forms() {
        assert_eq!(
            said("You've hit your limit \u{b7} resets 10pm").time,
            time(22, 0)
        );
        assert_eq!(
            said("You've hit your limit \u{b7} resets 12am").time,
            time(0, 0)
        );
        assert_eq!(
            said("You've hit your limit \u{b7} resets 12:30pm").time,
            time(12, 30)
        );
        assert_eq!(
            said("You've hit your limit \u{b7} resets 22:40").time,
            time(22, 40)
        );
        assert_eq!(
            said("You've hit your limit \u{b7} resets 9:05 a.m.").time,
            time(9, 5)
        );
        assert_eq!(
            said("You've hit your limit \u{b7} resets at 7am").time,
            time(7, 0)
        );
        assert_eq!(
            said("You've hit your limit \u{b7} resets 10:40pm").zone,
            None
        );

        let dated = said("You've hit your weekly limit \u{b7} resets Nov 23, 2027, 10:40am (UTC)");
        assert_eq!(
            dated.date,
            Some(SaidDate {
                month: 11,
                day: 23,
                year: Some(2027)
            })
        );
        assert_eq!(dated.time, time(10, 40));
        assert_eq!(dated.zone.as_deref(), Some("UTC"));
    }

    #[test]
    fn a_reset_that_is_not_understood_is_unknown_and_the_limit_still_known() {
        for words in [
            "You've hit your 5-hour limit \u{b7} resets in 3h 5m",
            "You've hit your 5-hour limit \u{b7} resets soon",
            "You've hit your 5-hour limit \u{b7} resets 25:00",
            "You've hit your 5-hour limit \u{b7} resets 13pm",
            "You've hit your 5-hour limit \u{b7} resets Foo 3, 10am",
        ] {
            let hit = limit_in_words(words).unwrap_or_else(|| panic!("not read: {words}"));
            assert_eq!(hit.window, Some(PlanWindowKind::FiveHours), "{words}");
            assert_eq!(hit.resets, PlanReset::Unknown, "{words}");
        }
    }
}
