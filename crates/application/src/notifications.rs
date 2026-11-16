//! Which events of a project are worth a notification of the operating
//! system, and in which words
//! ([decision 0026](../../../docs/decisions/0026-notifications.md)).
//!
//! Notifications are off until the manager allows them. The choice is a
//! fact about one computer: it is kept there, never synced and never
//! logged. Whether the window is in front, and how a notification is sent,
//! is the shell's to know.

use std::sync::Arc;

use crate::error::ApplicationError;
use crate::ports::StorageError;
use crate::project_events::{Happening, ProjectEvent, WaitsFor};

/// Whether the manager has allowed notifications on this computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationPermission {
    /// The manager has not been asked yet. Nothing is sent.
    #[default]
    NotAsked,
    /// The manager allowed them.
    Allowed,
    /// The manager said no. Nothing is sent, and the program does not ask
    /// again; the manager can still allow them in the settings.
    Declined,
}

/// The three kinds of notification, each of which the manager can turn off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationKind {
    /// A specialist or the Advisor waits for the manager.
    Waiting,
    /// A specialist has finished its work on a task.
    Finished,
    /// A specialist or the Advisor could not go on.
    Stopped,
}

/// What the manager chose about notifications on this computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationChoice {
    /// Whether they are allowed at all.
    pub permission: NotificationPermission,
    /// Whether one is sent when someone waits for the manager.
    pub waiting: bool,
    /// Whether one is sent when work on a task is finished.
    pub finished: bool,
    /// Whether one is sent when someone could not go on.
    pub stopped: bool,
}

impl Default for NotificationChoice {
    /// Not asked yet, and every kind on, for when they are allowed.
    fn default() -> Self {
        Self {
            permission: NotificationPermission::NotAsked,
            waiting: true,
            finished: true,
            stopped: true,
        }
    }
}

impl NotificationChoice {
    /// Whether a notification of this kind is sent.
    #[must_use]
    pub const fn sends(&self, kind: NotificationKind) -> bool {
        matches!(self.permission, NotificationPermission::Allowed)
            && match kind {
                NotificationKind::Waiting => self.waiting,
                NotificationKind::Finished => self.finished,
                NotificationKind::Stopped => self.stopped,
            }
    }

    /// The same choice with one kind turned on or off.
    #[must_use]
    pub const fn with_kind(mut self, kind: NotificationKind, on: bool) -> Self {
        match kind {
            NotificationKind::Waiting => self.waiting = on,
            NotificationKind::Finished => self.finished = on,
            NotificationKind::Stopped => self.stopped = on,
        }
        self
    }
}

/// A notification, worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    /// Its kind.
    pub kind: NotificationKind,
    /// The line in bold: who and what.
    pub title: String,
    /// The line under it: the question, the task, the reason.
    pub body: String,
}

/// The most characters the line under the title has; a longer text is cut
/// at a word and ends in an ellipsis.
pub const MOST_BODY_CHARACTERS: usize = 120;

/// The notification an event is worth, whatever the manager chose, or
/// nothing: the manager is told when someone waits for them, when work on
/// a task is finished and when someone could not go on, and of nothing
/// else.
#[must_use]
pub fn notification_of(event: &ProjectEvent) -> Option<Notification> {
    let (kind, title, body) = match &event.happening {
        Happening::WaitsForManager {
            agent, waits_for, ..
        } => {
            let name = agent.name();
            let (title, body) = match waits_for {
                WaitsFor::Decision { question } => {
                    (format!("{name} asks you to decide"), first_words(question))
                }
                WaitsFor::Answers { first_question, .. } => (
                    format!("{name} has questions for you"),
                    first_words(first_question),
                ),
                WaitsFor::Approval { members, tasks } => (
                    format!("{name} proposes a plan"),
                    format!(
                        "{} and {} for {}, for you to approve.",
                        counted(*members, "member", "members"),
                        counted(*tasks, "task", "tasks"),
                        event.project_name
                    ),
                ),
            };
            (NotificationKind::Waiting, title, body)
        }
        Happening::TaskFinished { task, .. } => (
            NotificationKind::Finished,
            format!("{} is ready", task.reference),
            first_words(&task.title),
        ),
        Happening::AgentStopped { agent, reason, .. } => (
            NotificationKind::Stopped,
            format!("{} stopped", agent.name()),
            if reason.is_empty() {
                "It could not go on.".to_owned()
            } else {
                first_words(reason)
            },
        ),
        Happening::SprintStarted { .. } | Happening::SprintCompleted { .. } => return None,
    };
    Some(Notification { kind, title, body })
}

/// The notification an event is worth with the choice the manager made:
/// nothing if notifications are not allowed, or if its kind is off.
#[must_use]
pub fn notification_for(event: &ProjectEvent, choice: &NotificationChoice) -> Option<Notification> {
    notification_of(event).filter(|notification| choice.sends(notification.kind))
}

/// The notification sent when the manager asks for one to try them.
#[must_use]
pub fn test_notification() -> Notification {
    Notification {
        kind: NotificationKind::Waiting,
        title: "Notifications are on".to_owned(),
        body: "Teamwright tells you here when a specialist needs you or is ready.".to_owned(),
    }
}

/// The first words of a text, on one line: at most
/// [`MOST_BODY_CHARACTERS`] characters, cut at a word.
fn first_words(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= MOST_BODY_CHARACTERS {
        return line;
    }
    let mut cut = String::new();
    for word in line.split(' ') {
        let longer = cut.chars().count() + usize::from(!cut.is_empty()) + word.chars().count();
        if longer > MOST_BODY_CHARACTERS - 1 {
            break;
        }
        if !cut.is_empty() {
            cut.push(' ');
        }
        cut.push_str(word);
    }
    if cut.is_empty() {
        // One word longer than the line: cut inside it.
        cut = line.chars().take(MOST_BODY_CHARACTERS - 1).collect();
    }
    cut.push('…');
    cut
}

/// A count with the word for one or for more.
fn counted(count: usize, one: &str, more: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { more })
}

/// Keeps the choice of this computer about notifications.
pub trait NotificationChoiceStore: Send + Sync {
    /// The choice, or the default one if the manager never made one.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be read.
    fn choice(&self) -> Result<NotificationChoice, StorageError>;

    /// Keeps a choice in place of the one before.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be kept.
    fn set_choice(&self, choice: &NotificationChoice) -> Result<(), StorageError>;
}

/// The use cases of notifications: the choice of this computer, and what an
/// event is worth with it.
#[derive(Clone)]
pub struct NotificationService {
    store: Arc<dyn NotificationChoiceStore>,
}

impl NotificationService {
    /// Creates the service on a store of the choice.
    #[must_use]
    pub fn new(store: Arc<dyn NotificationChoiceStore>) -> Self {
        Self { store }
    }

    /// The choice of this computer.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be read.
    pub fn choice(&self) -> Result<NotificationChoice, ApplicationError> {
        Ok(self.store.choice()?)
    }

    /// Allows notifications, with the kinds as they were.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be read or kept.
    pub fn allow(&self) -> Result<NotificationChoice, ApplicationError> {
        self.change(|choice| NotificationChoice {
            permission: NotificationPermission::Allowed,
            ..choice
        })
    }

    /// Turns notifications off, or says no to the question whether to allow
    /// them. Either way the program does not ask again.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be read or kept.
    pub fn decline(&self) -> Result<NotificationChoice, ApplicationError> {
        self.change(|choice| NotificationChoice {
            permission: NotificationPermission::Declined,
            ..choice
        })
    }

    /// Turns one kind of notification on or off.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be read or kept.
    pub fn set_kind(
        &self,
        kind: NotificationKind,
        on: bool,
    ) -> Result<NotificationChoice, ApplicationError> {
        self.change(|choice| choice.with_kind(kind, on))
    }

    /// The notification an event is worth with the choice of this computer,
    /// or nothing.
    ///
    /// # Errors
    ///
    /// Returns an error if the choice could not be read.
    pub fn notification_for(
        &self,
        event: &ProjectEvent,
    ) -> Result<Option<Notification>, ApplicationError> {
        Ok(notification_for(event, &self.store.choice()?))
    }

    /// The notification to try them with, if they are allowed.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::NotificationsOff`] if they are not
    /// allowed, or an error if the choice could not be read.
    pub fn test_notification(&self) -> Result<Notification, ApplicationError> {
        if self.store.choice()?.permission == NotificationPermission::Allowed {
            Ok(test_notification())
        } else {
            Err(ApplicationError::NotificationsOff)
        }
    }

    fn change(
        &self,
        change: impl FnOnce(NotificationChoice) -> NotificationChoice,
    ) -> Result<NotificationChoice, ApplicationError> {
        let choice = change(self.store.choice()?);
        self.store.set_choice(&choice)?;
        Ok(choice)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use teamwright_domain::{ConversationId, ProjectId, TaskId};

    use super::*;
    use crate::in_memory::InMemoryNotificationChoice;
    use crate::project_events::{Agent, TaskOfEvent};

    fn event(happening: Happening) -> ProjectEvent {
        ProjectEvent {
            project_id: ProjectId::generate(),
            project_name: "Smart sensor node".to_owned(),
            at: chrono::Utc.with_ymd_and_hms(2026, 11, 16, 9, 0, 0).unwrap(),
            happening,
        }
    }

    fn engineer() -> Agent {
        Agent::Specialist {
            member_id: None,
            name: "Hardware engineer".to_owned(),
        }
    }

    fn asks(question: &str) -> ProjectEvent {
        event(Happening::WaitsForManager {
            conversation_id: ConversationId::generate(),
            agent: engineer(),
            task: None,
            waits_for: WaitsFor::Decision {
                question: question.to_owned(),
            },
        })
    }

    fn finishes() -> ProjectEvent {
        event(Happening::TaskFinished {
            conversation_id: ConversationId::generate(),
            specialist: engineer(),
            task: TaskOfEvent {
                task_id: TaskId::generate(),
                reference: "SH-4".to_owned(),
                title: "Draw the schematic".to_owned(),
            },
            report: "Done.".to_owned(),
        })
    }

    fn allowed() -> NotificationChoice {
        NotificationChoice {
            permission: NotificationPermission::Allowed,
            ..NotificationChoice::default()
        }
    }

    #[test]
    fn a_question_is_told_with_its_first_words() {
        let notification = notification_of(&asks("Which regulator?\nThe board is small.")).unwrap();
        assert_eq!(notification.kind, NotificationKind::Waiting);
        assert_eq!(notification.title, "Hardware engineer asks you to decide");
        assert_eq!(notification.body, "Which regulator? The board is small.");

        let long = "word ".repeat(60);
        let body = notification_of(&asks(&long)).unwrap().body;
        assert!(body.chars().count() <= MOST_BODY_CHARACTERS);
        assert!(body.ends_with("word…"), "{body}");
    }

    #[test]
    fn finished_work_and_a_stop_are_told_in_few_words() {
        let notification = notification_of(&finishes()).unwrap();
        assert_eq!(notification.kind, NotificationKind::Finished);
        assert_eq!(notification.title, "SH-4 is ready");
        assert_eq!(notification.body, "Draw the schematic");

        let stopped = notification_of(&event(Happening::AgentStopped {
            conversation_id: ConversationId::generate(),
            agent: Agent::Advisor,
            task: None,
            reason: String::new(),
        }))
        .unwrap();
        assert_eq!(stopped.kind, NotificationKind::Stopped);
        assert_eq!(stopped.title, "The Advisor stopped");
        assert_eq!(stopped.body, "It could not go on.");

        let proposal = notification_of(&event(Happening::WaitsForManager {
            conversation_id: ConversationId::generate(),
            agent: Agent::Advisor,
            task: None,
            waits_for: WaitsFor::Approval {
                members: 1,
                tasks: 7,
            },
        }))
        .unwrap();
        assert_eq!(proposal.title, "The Advisor proposes a plan");
        assert_eq!(
            proposal.body,
            "1 member and 7 tasks for Smart sensor node, for you to approve."
        );
    }

    #[test]
    fn a_sprint_is_not_worth_a_notification() {
        let started = event(Happening::SprintStarted {
            sprint_id: teamwright_domain::SprintId::generate(),
            name: "Sprint 1".to_owned(),
            goal: String::new(),
        });
        assert_eq!(notification_of(&started), None);
    }

    #[test]
    fn nothing_is_sent_until_allowed_nor_of_a_kind_turned_off() {
        assert_eq!(
            notification_for(&finishes(), &NotificationChoice::default()),
            None
        );
        let declined = NotificationChoice {
            permission: NotificationPermission::Declined,
            ..NotificationChoice::default()
        };
        assert_eq!(notification_for(&finishes(), &declined), None);
        assert!(notification_for(&finishes(), &allowed()).is_some());
        let without_finished = allowed().with_kind(NotificationKind::Finished, false);
        assert_eq!(notification_for(&finishes(), &without_finished), None);
        assert!(notification_for(&asks("Which?"), &without_finished).is_some());
    }

    #[test]
    fn the_service_keeps_the_choice_and_tries_only_when_allowed() {
        let service = NotificationService::new(Arc::new(InMemoryNotificationChoice::default()));
        assert_eq!(service.choice().unwrap(), NotificationChoice::default());
        assert!(matches!(
            service.test_notification(),
            Err(ApplicationError::NotificationsOff)
        ));
        assert_eq!(service.notification_for(&finishes()).unwrap(), None);

        service.allow().unwrap();
        service.set_kind(NotificationKind::Stopped, false).unwrap();
        let choice = service.choice().unwrap();
        assert_eq!(choice.permission, NotificationPermission::Allowed);
        assert!(!choice.stopped && choice.waiting && choice.finished);
        assert!(service.test_notification().is_ok());
        assert!(service.notification_for(&finishes()).unwrap().is_some());

        let declined = service.decline().unwrap();
        assert_eq!(declined.permission, NotificationPermission::Declined);
        // The kinds stay as they were, for when they are allowed again.
        assert!(!declined.stopped);
    }
}
