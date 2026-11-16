//! What the listeners outside the computer are told of an event: the kind
//! of an event, the one plain line a chat channel gets, and the payload a
//! web address gets. The functions here are pure.

use teamwright_domain::{EventKind, ProjectId, SprintId, SprintUnit, Timestamp};

use crate::notifications::first_words;
use crate::project_events::{Agent, Happening, ProjectEvent, StatusOfEvent, TaskOfEvent, WaitsFor};

/// The kind of an event, by which the manager chooses what is passed on.
#[must_use]
pub const fn kind_of(happening: &Happening) -> EventKind {
    match happening {
        Happening::TaskCreated { .. } => EventKind::TaskCreated,
        Happening::TaskStatusChanged { .. } => EventKind::TaskStatusChanged,
        Happening::TaskFinished { .. } => EventKind::TaskFinished,
        Happening::WaitsForManager { .. } => EventKind::WaitsForManager,
        Happening::AgentStopped { .. } => EventKind::AgentStopped,
        Happening::SprintStarted { .. } => EventKind::SprintStarted,
        Happening::SprintCompleted { .. } => EventKind::SprintCompleted,
    }
}

/// A task as a sentence names it: its reference and its title.
fn named(task: &TaskOfEvent) -> String {
    format!("{} {}", task.reference, task.title)
}

/// The one plain line a chat channel is told of an event: the project's
/// name first, then what happened, with the task's reference and title and
/// who. No decoration.
#[must_use]
pub fn channel_message_of(event: &ProjectEvent) -> String {
    let project = &event.project_name;
    let what = match &event.happening {
        Happening::TaskCreated { task } => match &task.assignee {
            Some(assignee) => format!("{} was opened, for {assignee}.", named(task)),
            None => format!("{} was opened.", named(task)),
        },
        Happening::TaskStatusChanged { task, from } => match &task.status {
            Some(to) => format!("{} moved from {} to {}.", named(task), from.name, to.name),
            None => format!("{} moved from {}.", named(task), from.name),
        },
        Happening::TaskFinished {
            specialist, task, ..
        } => format!("{} finished {}.", specialist.name(), named(task)),
        Happening::WaitsForManager {
            agent,
            task,
            waits_for,
            ..
        } => {
            let on = task
                .as_ref()
                .map(|task| format!(" on {}", named(task)))
                .unwrap_or_default();
            let what = match waits_for {
                WaitsFor::Decision { question } => format!("a decision: {}", first_words(question)),
                WaitsFor::Answers { first_question, .. } => {
                    format!("answers: {}", first_words(first_question))
                }
                WaitsFor::Approval { members, tasks } => format!(
                    "an approval of a plan of {} and {}.",
                    counted(*members, "member", "members"),
                    counted(*tasks, "task", "tasks")
                ),
            };
            format!("{} waits for you{on}, for {what}", agent.name())
        }
        Happening::AgentStopped {
            agent,
            task,
            reason,
            ..
        } => {
            let on = task
                .as_ref()
                .map(|task| format!(" on {}", named(task)))
                .unwrap_or_default();
            if reason.is_empty() {
                format!("{} stopped{on}.", agent.name())
            } else {
                format!("{} stopped{on}: {}", agent.name(), first_words(reason))
            }
        }
        Happening::SprintStarted { name, goal, .. } => {
            if goal.trim().is_empty() {
                format!("{name} started.")
            } else {
                format!("{name} started. Its goal: {}", first_words(goal))
            }
        }
        Happening::SprintCompleted {
            name,
            unit,
            committed,
            completed,
            ..
        } => {
            let unit = match unit {
                SprintUnit::Points => "points",
                SprintUnit::Tasks => "tasks",
            };
            format!("{name} was completed: {completed} of {committed} {unit} done.")
        }
    };
    format!("{project}: {what}")
}

/// The line a chat channel gets to try it.
#[must_use]
pub fn channel_test_message(project_name: &str) -> String {
    format!("{project_name}: Teamwright will tell this channel what happens in the project.")
}

fn counted(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// The version of the shape of what a web address gets. It changes only
/// when a field is taken away or changes its meaning; a new field does not
/// change it.
pub const PAYLOAD_VERSION: u32 = 1;

/// What a web address gets of an event, in plain data. Its JSON form is
/// documented in `docs/integrations/webhooks.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPayload {
    /// The stable name of the event, such as `task.status_changed`, or
    /// `test` for a test.
    pub event: &'static str,
    /// When it happened.
    pub at: Timestamp,
    /// The project.
    pub project_id: ProjectId,
    /// The project's key, such as `SH`.
    pub project_key: String,
    /// The project's name.
    pub project_name: String,
    /// The task, where there is one.
    pub task: Option<TaskOfEvent>,
    /// The specialist or the Advisor, where the event is about one.
    pub agent: Option<Agent>,
    /// The status a task left, for `task.status_changed`.
    pub from_status: Option<StatusOfEvent>,
    /// What an agent waits for (`decision`, `answers` or `approval`), for
    /// `agent.waiting`.
    pub waits_for: Option<&'static str>,
    /// The first words of the question, for `agent.waiting`.
    pub question: Option<String>,
    /// Why an agent stopped, for `agent.stopped`.
    pub reason: Option<String>,
    /// The sprint, for `sprint.started` and `sprint.completed`.
    pub sprint: Option<SprintOfPayload>,
}

/// A sprint as a payload names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprintOfPayload {
    /// The sprint.
    pub sprint_id: SprintId,
    /// Its name.
    pub name: String,
    /// Its goal, when it started.
    pub goal: Option<String>,
    /// What its work is counted in, when it was completed.
    pub unit: Option<SprintUnit>,
    /// The work it took on, when it was completed.
    pub committed: Option<u32>,
    /// The work of it that was finished, when it was completed.
    pub completed: Option<u32>,
}

/// What a web address gets of an event. `project_key` is the key of the
/// event's project.
#[must_use]
pub fn payload_of(event: &ProjectEvent, project_key: &str) -> EventPayload {
    let mut payload = EventPayload {
        event: kind_of(&event.happening).as_str(),
        at: event.at,
        project_id: event.project_id,
        project_key: project_key.to_owned(),
        project_name: event.project_name.clone(),
        task: None,
        agent: None,
        from_status: None,
        waits_for: None,
        question: None,
        reason: None,
        sprint: None,
    };
    match &event.happening {
        Happening::TaskCreated { task } => payload.task = Some(task.clone()),
        Happening::TaskStatusChanged { task, from } => {
            payload.task = Some(task.clone());
            payload.from_status = Some(from.clone());
        }
        Happening::TaskFinished {
            specialist, task, ..
        } => {
            payload.task = Some(task.clone());
            payload.agent = Some(specialist.clone());
        }
        Happening::WaitsForManager {
            agent,
            task,
            waits_for,
            ..
        } => {
            payload.task.clone_from(task);
            payload.agent = Some(agent.clone());
            let (kind, question) = match waits_for {
                WaitsFor::Decision { question } => ("decision", Some(first_words(question))),
                WaitsFor::Answers { first_question, .. } => {
                    ("answers", Some(first_words(first_question)))
                }
                WaitsFor::Approval { .. } => ("approval", None),
            };
            payload.waits_for = Some(kind);
            payload.question = question;
        }
        Happening::AgentStopped {
            agent,
            task,
            reason,
            ..
        } => {
            payload.task.clone_from(task);
            payload.agent = Some(agent.clone());
            payload.reason = Some(first_words(reason));
        }
        Happening::SprintStarted {
            sprint_id,
            name,
            goal,
        } => {
            payload.sprint = Some(SprintOfPayload {
                sprint_id: *sprint_id,
                name: name.clone(),
                goal: Some(goal.clone()),
                unit: None,
                committed: None,
                completed: None,
            });
        }
        Happening::SprintCompleted {
            sprint_id,
            name,
            unit,
            committed,
            completed,
        } => {
            payload.sprint = Some(SprintOfPayload {
                sprint_id: *sprint_id,
                name: name.clone(),
                goal: None,
                unit: Some(*unit),
                committed: Some(*committed),
                completed: Some(*completed),
            });
        }
    }
    payload
}

/// What a web address gets to try it: the event `test`, with the project
/// and nothing else.
#[must_use]
pub fn test_payload(
    project_id: ProjectId,
    project_key: &str,
    project_name: &str,
    at: Timestamp,
) -> EventPayload {
    EventPayload {
        event: "test",
        at,
        project_id,
        project_key: project_key.to_owned(),
        project_name: project_name.to_owned(),
        task: None,
        agent: None,
        from_status: None,
        waits_for: None,
        question: None,
        reason: None,
        sprint: None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use teamwright_domain::{ConversationId, MemberId, StatusCategory, TaskId};

    use super::*;

    fn event(happening: Happening) -> ProjectEvent {
        ProjectEvent {
            project_id: ProjectId::generate(),
            project_name: "Smart sensor node".to_owned(),
            at: chrono::Utc
                .with_ymd_and_hms(2026, 11, 16, 9, 30, 0)
                .unwrap(),
            happening,
        }
    }

    fn status(name: &str, category: StatusCategory) -> StatusOfEvent {
        StatusOfEvent {
            name: name.to_owned(),
            category,
        }
    }

    fn task() -> TaskOfEvent {
        TaskOfEvent {
            task_id: TaskId::generate(),
            reference: "SH-4".to_owned(),
            title: "Draw the schematic".to_owned(),
            status: Some(status("In test", StatusCategory::Review)),
            assignee: Some("Hardware engineer".to_owned()),
        }
    }

    fn engineer() -> Agent {
        Agent::Specialist {
            member_id: Some(MemberId::generate()),
            name: "Hardware engineer".to_owned(),
        }
    }

    #[test]
    fn a_channel_is_told_one_plain_line() {
        let cases = [
            (
                Happening::TaskCreated { task: task() },
                "Smart sensor node: SH-4 Draw the schematic was opened, for Hardware engineer.",
            ),
            (
                Happening::TaskStatusChanged {
                    task: task(),
                    from: status("To do", StatusCategory::Todo),
                },
                "Smart sensor node: SH-4 Draw the schematic moved from To do to In test.",
            ),
            (
                Happening::TaskFinished {
                    conversation_id: ConversationId::generate(),
                    specialist: engineer(),
                    task: task(),
                    report: "Done.".to_owned(),
                },
                "Smart sensor node: Hardware engineer finished SH-4 Draw the schematic.",
            ),
            (
                Happening::WaitsForManager {
                    conversation_id: ConversationId::generate(),
                    agent: engineer(),
                    task: Some(task()),
                    waits_for: WaitsFor::Decision {
                        question: "Which regulator?".to_owned(),
                    },
                },
                "Smart sensor node: Hardware engineer waits for you on SH-4 Draw the schematic, \
                 for a decision: Which regulator?",
            ),
            (
                Happening::AgentStopped {
                    conversation_id: ConversationId::generate(),
                    agent: Agent::Advisor,
                    task: None,
                    reason: "The program ran out of time.".to_owned(),
                },
                "Smart sensor node: The Advisor stopped: The program ran out of time.",
            ),
            (
                Happening::SprintStarted {
                    sprint_id: SprintId::generate(),
                    name: "Sprint 2".to_owned(),
                    goal: String::new(),
                },
                "Smart sensor node: Sprint 2 started.",
            ),
            (
                Happening::SprintCompleted {
                    sprint_id: SprintId::generate(),
                    name: "Sprint 2".to_owned(),
                    unit: SprintUnit::Points,
                    committed: 21,
                    completed: 18,
                },
                "Smart sensor node: Sprint 2 was completed: 18 of 21 points done.",
            ),
        ];
        for (happening, expected) in cases {
            assert_eq!(channel_message_of(&event(happening)), expected);
        }
    }

    #[test]
    fn every_happening_has_its_kind() {
        assert_eq!(
            kind_of(&Happening::TaskCreated { task: task() }),
            EventKind::TaskCreated
        );
        assert_eq!(
            kind_of(&Happening::SprintStarted {
                sprint_id: SprintId::generate(),
                name: String::new(),
                goal: String::new()
            })
            .as_str(),
            "sprint.started"
        );
    }

    #[test]
    fn a_payload_carries_what_its_event_has() {
        let changed = payload_of(
            &event(Happening::TaskStatusChanged {
                task: task(),
                from: status("To do", StatusCategory::Todo),
            }),
            "SH",
        );
        assert_eq!(changed.event, "task.status_changed");
        assert_eq!(changed.project_key, "SH");
        assert_eq!(changed.task.unwrap().reference, "SH-4");
        assert_eq!(changed.from_status.unwrap().name, "To do");
        assert_eq!(changed.agent, None);

        let waiting = payload_of(
            &event(Happening::WaitsForManager {
                conversation_id: ConversationId::generate(),
                agent: Agent::Advisor,
                task: None,
                waits_for: WaitsFor::Approval {
                    members: 3,
                    tasks: 12,
                },
            }),
            "SH",
        );
        assert_eq!(waiting.waits_for, Some("approval"));
        assert_eq!(waiting.question, None);
        assert_eq!(waiting.agent, Some(Agent::Advisor));

        let completed = payload_of(
            &event(Happening::SprintCompleted {
                sprint_id: SprintId::generate(),
                name: "Sprint 2".to_owned(),
                unit: SprintUnit::Tasks,
                committed: 8,
                completed: 7,
            }),
            "SH",
        );
        let sprint = completed.sprint.unwrap();
        assert_eq!(
            (sprint.unit, sprint.committed, sprint.completed),
            (Some(SprintUnit::Tasks), Some(8), Some(7))
        );
    }
}
