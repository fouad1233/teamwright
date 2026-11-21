//! Telling the manager that a newer version of the program is out
//! ([decision 0043](../../../docs/decisions/0043-a-notice-of-newer-versions.md)).
//!
//! The program only notices and tells where to get the version. It does not
//! install anything. Looking is a call out to the public list of releases,
//! so it is off until the manager switches it on, happens at most once a
//! day by itself, and happens once more whenever the manager presses
//! "Look now". The choice and the time of the last look are facts about one
//! computer: kept there, never logged, never synced.

use std::sync::Arc;

use chrono::TimeDelta;
use teamwright_domain::{Release, Timestamp, Version, newer_release};

use crate::error::ApplicationError;
use crate::ports::{Clock, StorageError};

/// How long after a look the program waits before it looks by itself again.
pub const LOOK_EVERY: TimeDelta = TimeDelta::hours(24);

/// Where the published releases are read from. A failure is never shown to
/// the manager as an error: the use case turns it into "could not look".
pub trait ReleaseSource: Send + Sync {
    /// The releases that are published, in any order. A release whose tag
    /// is not a version is left out.
    ///
    /// # Errors
    ///
    /// Why the list could not be had, in words that do not repeat an
    /// address.
    fn published_releases(&self) -> Result<Vec<Release>, String>;
}

/// What the manager chose and what the last look found, on this computer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateChoice {
    /// Whether the program looks by itself. Off until switched on.
    pub is_on: bool,
    /// When the program last looked, by itself or because it was asked.
    pub last_look: Option<Timestamp>,
    /// Whether the last look could not get an answer.
    pub last_look_failed: bool,
    /// The newest release the last answering look offered.
    pub offered: Option<Release>,
}

/// Keeps the choice about looking for newer versions.
pub trait UpdateChoiceStore: Send + Sync {
    /// The choice, or the defaults if nothing was ever kept.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be read.
    fn choice(&self) -> Result<UpdateChoice, StorageError>;

    /// Switches looking by itself on or off.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be kept.
    fn set_on(&self, is_on: bool) -> Result<(), StorageError>;

    /// Keeps that a look happened at a moment. `Some` is what it found to
    /// offer (nothing offered is `Some(None)`); `None` means it got no
    /// answer, and what was offered before stays.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be kept.
    fn record_look(
        &self,
        at: Timestamp,
        answer: Option<Option<&Release>>,
    ) -> Result<(), StorageError>;
}

/// What the interface shows about versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateStatus {
    /// The version of this program, as it reads.
    pub this_version: String,
    /// Whether the program looks by itself.
    pub is_on: bool,
    /// When it last looked.
    pub last_look: Option<Timestamp>,
    /// Whether the last look got no answer.
    pub last_look_failed: bool,
    /// The newer release that is known, if any.
    pub newer: Option<Release>,
}

/// The use cases of noticing newer versions.
#[derive(Clone)]
pub struct UpdateService {
    current: Option<Version>,
    this_version: String,
    store: Arc<dyn UpdateChoiceStore>,
    source: Arc<dyn ReleaseSource>,
    clock: Arc<dyn Clock>,
}

impl UpdateService {
    /// Creates the service for a program of the given version.
    #[must_use]
    pub fn new(
        this_version: &str,
        store: Arc<dyn UpdateChoiceStore>,
        source: Arc<dyn ReleaseSource>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            current: Version::parse(this_version),
            this_version: this_version.to_owned(),
            store,
            source,
            clock,
        }
    }

    /// What is known now, without looking.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be read.
    pub fn status(&self) -> Result<UpdateStatus, ApplicationError> {
        let choice = self.store.choice()?;
        // A release that is no newer than this program, because the manager
        // installed it since the look, is no longer news.
        let newer = match (&self.current, choice.offered) {
            (Some(current), Some(offered)) if offered.version > *current => Some(offered),
            _ => None,
        };
        Ok(UpdateStatus {
            this_version: self.this_version.clone(),
            is_on: choice.is_on,
            last_look: choice.last_look,
            last_look_failed: choice.last_look_failed,
            newer,
        })
    }

    /// Switches looking by itself on or off.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be kept.
    pub fn switch(&self, is_on: bool) -> Result<UpdateStatus, ApplicationError> {
        self.store.set_on(is_on)?;
        self.status()
    }

    /// Looks if the manager switched it on and the last look is a day old
    /// or older; otherwise only reports what is known. The program calls
    /// this when it starts and now and then while it runs.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be read or kept. A look that
    /// got no answer is not an error.
    pub fn look_if_due(&self) -> Result<UpdateStatus, ApplicationError> {
        let choice = self.store.choice()?;
        let now = self.clock.now();
        let is_due = choice
            .last_look
            .is_none_or(|last| last > now || now - last >= LOOK_EVERY);
        if choice.is_on && is_due {
            self.look(now)?;
        }
        self.status()
    }

    /// Looks now, because the manager pressed the button, whether looking by
    /// itself is on or not.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be kept. A look that got no
    /// answer is not an error: the status says so.
    pub fn look_now(&self) -> Result<UpdateStatus, ApplicationError> {
        self.look(self.clock.now())?;
        self.status()
    }

    fn look(&self, at: Timestamp) -> Result<(), ApplicationError> {
        let answer = match (&self.current, self.source.published_releases()) {
            (Some(current), Ok(published)) => Some(newer_release(current, &published).cloned()),
            // A program whose version is not one has nothing to compare:
            // it looks, and offers nothing.
            (None, Ok(_)) => Some(None),
            (_, Err(_)) => None,
        };
        self.store
            .record_look(at, answer.as_ref().map(Option::as_ref))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::TimeZone;
    use teamwright_domain::WebAddress;

    use super::*;
    use crate::in_memory::{InMemoryUpdateChoice, ManualClock};

    /// Answers with a list, or with a failure, and counts the looks.
    struct Source {
        answer: Mutex<Result<Vec<Release>, String>>,
        looks: Mutex<u32>,
    }

    impl Source {
        fn answering(answer: Result<Vec<Release>, String>) -> Arc<Self> {
            Arc::new(Self {
                answer: Mutex::new(answer),
                looks: Mutex::new(0),
            })
        }

        fn looks(&self) -> u32 {
            *self.looks.lock().unwrap()
        }
    }

    impl ReleaseSource for Source {
        fn published_releases(&self) -> Result<Vec<Release>, String> {
            *self.looks.lock().unwrap() += 1;
            self.answer.lock().unwrap().clone()
        }
    }

    fn release(tag: &str, is_pre_release: bool) -> Release {
        Release {
            version: Version::parse(tag).unwrap(),
            is_pre_release,
            address: WebAddress::parse(
                &format!("https://github.com/o/r/releases/tag/{tag}"),
                "release",
            )
            .unwrap(),
        }
    }

    fn morning() -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 21, 8, 0, 0).unwrap()
    }

    fn setup(
        this_version: &str,
        answer: Result<Vec<Release>, String>,
    ) -> (UpdateService, Arc<Source>, Arc<ManualClock>) {
        let source = Source::answering(answer);
        let clock = Arc::new(ManualClock::starting_at(morning()));
        let service = UpdateService::new(
            this_version,
            Arc::new(InMemoryUpdateChoice::default()),
            source.clone(),
            clock.clone(),
        );
        (service, source, clock)
    }

    #[test]
    fn nothing_is_looked_up_until_it_is_switched_on() {
        let (service, source, _) = setup("0.1.0", Ok(vec![release("v0.2.0", true)]));
        let status = service.look_if_due().unwrap();
        assert!(!status.is_on);
        assert_eq!(source.looks(), 0);
        assert_eq!(status.last_look, None);
        assert_eq!(status.newer, None);
        assert_eq!(status.this_version, "0.1.0");
    }

    #[test]
    fn once_switched_on_it_looks_and_then_at_most_once_a_day() {
        let (service, source, clock) = setup(
            "0.1.0",
            Ok(vec![release("v0.1.0", true), release("v0.2.0", true)]),
        );
        service.switch(true).unwrap();

        let status = service.look_if_due().unwrap();
        assert_eq!(source.looks(), 1);
        assert_eq!(status.newer.unwrap().version.text(), "0.2.0");
        assert_eq!(status.last_look, Some(morning()));

        clock.advance(TimeDelta::hours(23));
        let status = service.look_if_due().unwrap();
        assert_eq!(source.looks(), 1, "a second look within a day");
        assert!(status.newer.is_some(), "the known release stays");

        clock.advance(TimeDelta::hours(1));
        service.look_if_due().unwrap();
        assert_eq!(source.looks(), 2);
    }

    #[test]
    fn a_clock_set_back_does_not_stop_looking_for_good() {
        let (service, source, clock) = setup("0.1.0", Ok(vec![]));
        service.switch(true).unwrap();
        service.look_if_due().unwrap();
        clock.set(morning() - TimeDelta::days(3));
        service.look_if_due().unwrap();
        assert_eq!(source.looks(), 2);
    }

    #[test]
    fn the_button_looks_even_when_it_is_off_and_a_day_has_not_passed() {
        let (service, source, _) = setup("0.1.0", Ok(vec![release("v0.2.0", true)]));
        let first = service.look_now().unwrap();
        let second = service.look_now().unwrap();
        assert_eq!(source.looks(), 2);
        assert!(!first.is_on && !second.is_on);
        assert_eq!(second.newer.unwrap().version.text(), "0.2.0");
    }

    #[test]
    fn a_look_that_fails_says_so_and_keeps_what_was_known() {
        let (service, source, _) = setup("0.1.0", Ok(vec![release("v0.2.0", true)]));
        service.look_now().unwrap();
        *source.answer.lock().unwrap() = Err("no answer".to_owned());
        let status = service.look_now().unwrap();
        assert!(status.last_look_failed);
        assert_eq!(status.newer.unwrap().version.text(), "0.2.0");
        assert_eq!(status.last_look, Some(morning()));
    }

    #[test]
    fn a_failed_look_counts_as_a_look_for_the_daily_limit() {
        let (service, source, _) = setup("0.1.0", Err("no answer".to_owned()));
        service.switch(true).unwrap();
        service.look_if_due().unwrap();
        service.look_if_due().unwrap();
        assert_eq!(source.looks(), 1);
    }

    #[test]
    fn a_later_good_look_clears_the_failure() {
        let (service, source, _) = setup("0.1.0", Err("no answer".to_owned()));
        assert!(service.look_now().unwrap().last_look_failed);
        *source.answer.lock().unwrap() = Ok(vec![]);
        assert!(!service.look_now().unwrap().last_look_failed);
    }

    #[test]
    fn a_release_the_manager_has_since_installed_is_no_longer_news() {
        let store = Arc::new(InMemoryUpdateChoice::default());
        let source = Source::answering(Ok(vec![release("v0.2.0", true)]));
        let clock = Arc::new(ManualClock::starting_at(morning()));
        let before = UpdateService::new("0.1.0", store.clone(), source.clone(), clock.clone());
        assert!(before.look_now().unwrap().newer.is_some());
        let after = UpdateService::new("0.2.0", store, source, clock);
        assert_eq!(after.status().unwrap().newer, None);
    }

    #[test]
    fn a_program_of_no_known_version_offers_nothing() {
        let (service, _, _) = setup("dev", Ok(vec![release("v0.2.0", true)]));
        assert_eq!(service.look_now().unwrap().newer, None);
    }
}
