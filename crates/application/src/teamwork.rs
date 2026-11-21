//! What specialists hand each other through the team bus: the handoffs of
//! tasks and the messages between teammates.
//!
//! Both reach Teamwright as events of a turn and are recorded by the work
//! of the specialists ([`crate::work`]). This module answers the questions
//! about them that span several stored things: what waits for a member,
//! which handoff of a task counts, and how both are put into words for a
//! specialist. The coordinator, which decides when a turn starts, asks
//! [`messages_waiting_for`] whether a message waits.

use std::sync::Arc;

use teamwright_domain::{
    Handoff, Member, MemberId, MessageKind, Project, ProjectId, Task, TaskId, TeamMessage,
};

use crate::error::ApplicationError;
use crate::ports::{MemberRepository, ProjectRepository, TeamworkRepository};
use crate::work::reference_of;

/// Everything the specialists of a project handed each other, for the
/// manager to read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeamTalk {
    /// The handoffs of its tasks, the oldest first.
    pub handoffs: Vec<Handoff>,
    /// The messages between its members, the oldest first.
    pub messages: Vec<TeamMessage>,
}

/// The use cases that read what specialists hand each other.
pub struct TeamworkService {
    projects: Arc<dyn ProjectRepository>,
    members: Arc<dyn MemberRepository>,
    teamwork: Arc<dyn TeamworkRepository>,
}

impl TeamworkService {
    /// Creates the service.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        members: Arc<dyn MemberRepository>,
        teamwork: Arc<dyn TeamworkRepository>,
    ) -> Self {
        Self {
            projects,
            members,
            teamwork,
        }
    }

    /// Every handoff and every message of a project.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn talk_of_project(&self, project_id: ProjectId) -> Result<TeamTalk, ApplicationError> {
        if self.projects.find(project_id)?.is_none() {
            return Err(ApplicationError::NotFound {
                kind: "project",
                id: project_id.to_string(),
            });
        }
        Ok(TeamTalk {
            handoffs: self.teamwork.handoffs_of_project(project_id)?,
            messages: self.teamwork.messages_of_project(project_id)?,
        })
    }

    /// The messages that wait for a member: sent to it and not delivered
    /// yet, the oldest first. They are delivered with its next turn.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such member or if storage fails.
    pub fn messages_waiting_for(
        &self,
        member_id: MemberId,
    ) -> Result<Vec<TeamMessage>, ApplicationError> {
        let member = self
            .members
            .find(member_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "team member",
                id: member_id.to_string(),
            })?;
        let messages = self.teamwork.messages_of_project(member.project_id())?;
        Ok(messages_waiting_for(&messages, member_id))
    }
}

/// The messages among `messages` that wait for a member: sent to it and not
/// delivered yet, the oldest first.
#[must_use]
pub fn messages_waiting_for(messages: &[TeamMessage], member_id: MemberId) -> Vec<TeamMessage> {
    let mut waiting: Vec<TeamMessage> = messages
        .iter()
        .filter(|message| message.to() == member_id && message.is_waiting())
        .cloned()
        .collect();
    waiting.sort_by_key(|message| (message.sent_at(), message.id()));
    waiting
}

/// The handoff of a task that counts: the latest it published.
#[must_use]
pub fn latest_handoff_of(handoffs: &[Handoff], task_id: TaskId) -> Option<&Handoff> {
    handoffs
        .iter()
        .filter(|handoff| handoff.task_id() == task_id)
        .max_by_key(|handoff| (handoff.published_at(), handoff.id()))
}

/// What the tasks a task builds on handed over, as a part of its brief:
/// for each of `blockers` that has a handoff, its summary, its interface
/// contract and its files. Nothing if none has one.
#[must_use]
pub fn handoffs_in_words(
    project: &Project,
    blockers: &[&Task],
    handoffs: &[Handoff],
) -> Option<String> {
    let parts: Vec<String> = blockers
        .iter()
        .filter_map(|blocker| {
            let handoff = latest_handoff_of(handoffs, blocker.id())?;
            let content = handoff.content();
            let mut lines = vec![
                format!(
                    "From {}: {}",
                    reference_of(project, blocker),
                    blocker.title()
                ),
                content.summary().to_owned(),
            ];
            if !content.contract().is_empty() {
                lines.push(String::new());
                lines.push("Interface contract:".to_owned());
                lines.push(content.contract().to_owned());
            }
            if !content.files().is_empty() {
                lines.push(String::new());
                lines.push(
                    "Files, in the folder of the project (read them before you begin):".to_owned(),
                );
                lines.extend(
                    content
                        .files()
                        .iter()
                        .map(|file| format!("- {}", file.as_text())),
                );
            }
            Some(lines.join("\n"))
        })
        .collect();
    if parts.is_empty() {
        return None;
    }
    let mut paragraphs = vec!["What the tasks it builds on handed over:".to_owned()];
    paragraphs.extend(parts);
    Some(paragraphs.join("\n\n"))
}

/// The messages that wait for a specialist, as it reads them at the start
/// of a turn. `team` names the senders; a sender who has left the team is
/// called a former teammate. Nothing if no message waits.
#[must_use]
pub fn messages_in_words(messages: &[TeamMessage], team: &[Member]) -> Option<String> {
    if messages.is_empty() {
        return None;
    }
    let mut lines = vec![
        "Messages from your teammates arrived since you last worked. Answer a question with the \
         tool send_message when you can; it reaches the teammate at the start of its next turn."
            .to_owned(),
    ];
    for message in messages {
        let sender = team
            .iter()
            .find(|member| member.id() == message.from())
            .map_or_else(
                || "A former teammate".to_owned(),
                |member| member.name().to_string(),
            );
        let kind = match message.kind() {
            MessageKind::Question => "asks",
            MessageKind::Note => "notes",
        };
        lines.push(String::new());
        lines.push(format!("{sender} {kind}:"));
        lines.push(message.text().to_owned());
    }
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone, Utc};
    use teamwright_domain::{
        ConversationId, HandoffContent, MemberKind, MemberName, ProjectKey, ProjectName, SaidIn,
        TaskDraft, TaskTitle, Timestamp, TurnId, Workflow,
    };

    use super::*;
    use crate::in_memory::{
        InMemoryMemberRepository, InMemoryProjectRepository, InMemoryTeamworkRepository,
    };

    fn at(minutes: i64) -> Timestamp {
        Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap() + TimeDelta::minutes(minutes)
    }

    fn project() -> Project {
        Project::create(
            ProjectName::parse("Sensor node").unwrap(),
            ProjectKey::parse("SSN").unwrap(),
            String::new(),
            Workflow::standard(),
            at(0),
        )
    }

    fn member(project: &Project, name: &str) -> Member {
        Member::create(
            project.id(),
            MemberName::parse(name).unwrap(),
            MemberKind::Agent,
            String::new(),
            teamwright_domain::Color::Teal,
            at(0),
        )
    }

    fn said_by(member: &Member) -> SaidIn {
        SaidIn {
            member_id: member.id(),
            conversation_id: ConversationId::generate(),
            turn_id: TurnId::generate(),
        }
    }

    fn message(from: &Member, to: &Member, text: &str, minute: i64) -> TeamMessage {
        TeamMessage::send(
            from.project_id(),
            said_by(from),
            to.id(),
            MessageKind::Question,
            text,
            at(minute),
        )
        .unwrap()
    }

    #[test]
    fn the_messages_that_wait_for_a_member_are_its_undelivered_ones_the_oldest_first() {
        let project = project();
        let hardware = member(&project, "Hardware engineer");
        let firmware = member(&project, "Firmware engineer");
        let later = message(&hardware, &firmware, "Second", 5);
        let earlier = message(&hardware, &firmware, "First", 1);
        let mut delivered = message(&hardware, &firmware, "Old", 0);
        delivered.deliver(TurnId::generate(), at(2));
        let to_someone_else = message(&firmware, &hardware, "Not for firmware", 3);

        let waiting = messages_waiting_for(
            &[later.clone(), delivered, to_someone_else, earlier.clone()],
            firmware.id(),
        );

        assert_eq!(waiting, [earlier, later]);
    }

    #[test]
    fn the_service_answers_which_messages_wait_for_a_member() {
        let project = project();
        let hardware = member(&project, "Hardware engineer");
        let firmware = member(&project, "Firmware engineer");
        let projects = Arc::new(InMemoryProjectRepository::default());
        let members = Arc::new(InMemoryMemberRepository::default());
        let teamwork = Arc::new(InMemoryTeamworkRepository::default());
        crate::ports::ProjectRepository::save(projects.as_ref(), &project).unwrap();
        crate::ports::MemberRepository::save(members.as_ref(), &hardware).unwrap();
        crate::ports::MemberRepository::save(members.as_ref(), &firmware).unwrap();
        let sent = message(&hardware, &firmware, "Which pins are free?", 1);
        teamwork.save_messages(std::slice::from_ref(&sent)).unwrap();
        let service = TeamworkService::new(projects, members, teamwork);

        assert_eq!(service.messages_waiting_for(firmware.id()).unwrap(), [sent]);
        assert!(
            service
                .messages_waiting_for(hardware.id())
                .unwrap()
                .is_empty()
        );
        assert!(service.messages_waiting_for(MemberId::generate()).is_err());
        assert_eq!(
            service
                .talk_of_project(project.id())
                .unwrap()
                .messages
                .len(),
            1
        );
        assert!(service.talk_of_project(ProjectId::generate()).is_err());
    }

    #[test]
    fn the_latest_handoff_of_a_task_counts_and_is_put_into_words_for_the_brief() {
        let project = project();
        let hardware = member(&project, "Hardware engineer");
        let pins = Task::create(
            project.id(),
            teamwright_domain::AreaId::generate(),
            3,
            &project.workflow().statuses()[0],
            teamwright_domain::Rank::between(None, None),
            TaskDraft::titled(TaskTitle::parse("Draw the pin map").unwrap()),
            at(0),
        );
        let publish = |summary: &str, contract: &str, files: &[&str], minute: i64| {
            Handoff::publish(
                project.id(),
                pins.id(),
                said_by(&hardware),
                HandoffContent::new(
                    summary,
                    contract,
                    &files
                        .iter()
                        .map(|file| (*file).to_owned())
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
                at(minute),
            )
        };
        let first = publish("A first draft.", "", &[], 1);
        let second = publish(
            "Pin map v1.",
            "Display on SPI2.",
            &["hardware/pinmap.md"],
            9,
        );
        let handoffs = [second.clone(), first];

        assert_eq!(latest_handoff_of(&handoffs, pins.id()), Some(&second));
        assert_eq!(latest_handoff_of(&handoffs, TaskId::generate()), None);
        assert_eq!(
            handoffs_in_words(&project, &[&pins], &handoffs).unwrap(),
            "What the tasks it builds on handed over:\n\n\
             From SSN-3: Draw the pin map\n\
             Pin map v1.\n\n\
             Interface contract:\n\
             Display on SPI2.\n\n\
             Files, in the folder of the project (read them before you begin):\n\
             - hardware/pinmap.md"
        );
        assert_eq!(handoffs_in_words(&project, &[&pins], &[]), None);
    }

    #[test]
    fn waiting_messages_are_put_into_words_with_their_senders() {
        let project = project();
        let hardware = member(&project, "Hardware engineer");
        let firmware = member(&project, "Firmware engineer");
        let gone = member(&project, "Tester");
        let question = message(&hardware, &firmware, "Which pins are free?", 1);
        let mut note = message(&gone, &firmware, "I left notes in test/.", 2);
        note = TeamMessage::restore(teamwright_domain::TeamMessageSnapshot {
            id: note.id(),
            project_id: note.project_id(),
            said_in: note.said_in(),
            to: note.to(),
            kind: MessageKind::Note,
            text: note.text().to_owned(),
            sent_at: note.sent_at(),
            delivery: None,
        })
        .unwrap();

        let words = messages_in_words(&[question, note], &[hardware, firmware]).unwrap();

        assert!(words.starts_with("Messages from your teammates arrived"));
        assert!(words.contains("Hardware engineer asks:\nWhich pins are free?"));
        assert!(words.contains("A former teammate notes:\nI left notes in test/."));
        assert_eq!(messages_in_words(&[], &[]), None);
    }
}
