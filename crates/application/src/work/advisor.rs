//! Planning a project with the Advisor
//! ([decision 0014](../../../../docs/decisions/0014-the-advisor-plans-through-two-tools.md)).
//!
//! The manager says what they want. The Advisor, a session that can only
//! read, asks what it needs to know in an interview, and then proposes the
//! members to add and the tasks to create. Nothing is created before the
//! manager approves, and approving creates everything in one write or
//! nothing.
//!
//! The Advisor is not a member of the team: its conversation has no member
//! and no task, and keeps the model and the effort it runs on.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use teamwright_domain::{
    Assignee, Conversation, ConversationId, Effort, InterviewAnswer, InterviewQuestion, Member,
    MemberId, MemberKind, MemberName, MemberProfile, ModelName, Priority, Project, ProjectId,
    Proposal, ProposedMember, ProposedOwner, ProposedTask, Rank, Schedule, Task, TaskDraft,
    TaskTitle, Turn, TurnOrigin,
};

use super::{
    ConversationWithTurns, NewTurn, REFERENCES_FOLDER, WorkService, Worker, open_interview_of,
    open_proposal_of, reference_of,
};
use crate::error::ApplicationError;
use crate::ports::ApprovedPlan;

/// A proposal as the manager approves it: the members and the tasks that
/// are left after the manager's changes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApprovedProposal {
    /// The members to add.
    pub members: Vec<ApprovedMember>,
    /// The tasks to create.
    pub tasks: Vec<ApprovedTask>,
}

/// A member of a proposal as the manager approves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedMember {
    /// What the member is to be called.
    pub name: String,
    /// What it is responsible for.
    pub responsibility: String,
    /// Why the project needs it.
    pub reason: String,
    /// The model, or none for the default of the program. An empty name
    /// means none.
    pub model: Option<String>,
    /// The effort, or none for the default of the program.
    pub effort: Option<Effort>,
}

/// A task of a proposal as the manager approves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedTask {
    /// What is to be done.
    pub title: String,
    /// What the result is, and how to tell that it is done.
    pub description: String,
    /// Who does it, or nobody yet.
    pub owner: Option<ApprovedOwner>,
    /// The size in minutes, if known.
    pub estimate_minutes: Option<u32>,
    /// How urgent it is.
    pub priority: Priority,
    /// The tasks of the same proposal that must be finished first, by
    /// their places from 0.
    pub waits_for: Vec<usize>,
}

/// Who a task of an approved proposal is given to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovedOwner {
    /// The manager.
    Manager,
    /// A member by name: a member of the proposal, or of the team.
    Member(String),
}

impl WorkService {
    /// Starts planning a project with the Advisor: a new conversation that
    /// begins with what the manager wants. The Advisor runs on this model
    /// and at this effort for the whole conversation, or on the defaults
    /// of the program; an empty model means none.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the brief is empty
    /// or the model is not one word (`rule_broken`), if the Advisor is
    /// working on the project already (`advisor_busy`), if the program
    /// cannot start the turn, or if storage fails.
    pub fn advise(
        &self,
        project_id: ProjectId,
        brief: &str,
        model: Option<&str>,
        effort: Option<Effort>,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        let _work = self.inner.hold_work();
        let ports = &self.inner.ports;
        let project = self.inner.require_project(project_id)?;
        let model = model
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .map(ModelName::parse)
            .transpose()?;
        let now = ports.clock.now();
        let conversation =
            Conversation::start_with_advisor(project.id(), model, effort, now, ports.this_computer);
        let turn = Turn::start(conversation.id(), brief, TurnOrigin::Message, now)?;
        self.inner
            .guard()
            .refuse_while_advisor_works(project.id())?;

        self.begin_turn(
            NewTurn {
                conversation,
                is_new_conversation: true,
                earlier_turns: Vec::new(),
                answered_turn: None,
                turn,
                prompt: None,
            },
            Worker::Advisor,
            &project,
        )
    }

    /// Answers the questions the Advisor asked in a conversation, one
    /// answer for each question, in their order: an option (counted from
    /// 0), words, or both. The Advisor goes on in the same session with the
    /// answers in words ([`answers_in_words`]) as the request of a new turn.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such conversation, if no questions
    /// are open in it (`no_interview_open`), if the answers do not fit the
    /// questions (`rule_broken`), if the Advisor works on the project
    /// (`advisor_busy`), if the conversation runs on another computer
    /// (`runs_elsewhere`), if the program cannot start the turn, or if
    /// storage fails. The answers are stored only if the new turn starts.
    pub fn answer_interview(
        &self,
        conversation_id: ConversationId,
        answers: Vec<InterviewAnswer>,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        let _work = self.inner.hold_work();
        let ports = &self.inner.ports;
        let conversation = self.inner.require_conversation_here(conversation_id)?;
        let mut earlier_turns = ports.conversations.turns_of(conversation.id())?;
        if open_interview_of(&earlier_turns).is_none() {
            return Err(ApplicationError::NoInterviewOpen);
        }
        let now = ports.clock.now();
        let asked = earlier_turns
            .last_mut()
            .ok_or(ApplicationError::NoInterviewOpen)?;
        asked.answer_interview(answers, now)?;
        self.inner
            .guard()
            .refuse_while_advisor_works(conversation.project_id())?;

        let words = asked
            .interview()
            .and_then(|interview| {
                Some(answers_in_words(
                    interview.questions(),
                    interview.answers()?,
                ))
            })
            .ok_or(ApplicationError::NoInterviewOpen)?;
        let answered_turn = asked.clone();
        let turn = Turn::start(conversation.id(), &words, TurnOrigin::Interview, now)?;
        let project = self.inner.require_project(conversation.project_id())?;
        self.begin_turn(
            NewTurn {
                conversation,
                is_new_conversation: false,
                earlier_turns,
                answered_turn: Some(answered_turn),
                turn,
                prompt: None,
            },
            Worker::Advisor,
            &project,
        )
    }

    /// Approves the proposal the Advisor made in the latest turn of a
    /// conversation, as the manager changed it.
    ///
    /// Everything is checked first. Then the members join the team as AI
    /// specialists, each with the color a newcomer gets, and the tasks are
    /// created in the first status of the workflow, after the tasks that
    /// are there, numbered on, each waiting for the tasks the proposal
    /// says. A proposed member whose name the team has already, whatever
    /// the capitals, is not added again, and a task given to that name
    /// goes to the member the team has. The project, the members, the
    /// tasks and the approved proposal are stored in one write, or
    /// nothing is. The proposal keeps what was approved, with the moment.
    /// No work starts by itself.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such conversation, if no proposal
    /// is open in it (`no_proposal_open`), if what is approved breaks a
    /// rule (`rule_broken`): a name or a title, two members of one name, a
    /// task given to somebody who is neither on the team nor in the
    /// proposal, a task waiting for no other task of the proposal, tasks
    /// waiting in a circle, or nothing left at all; if the conversation runs
    /// on another computer (`runs_elsewhere`), where approving it a second
    /// time would create the same tasks twice; or if storage fails.
    pub fn approve_proposal(
        &self,
        conversation_id: ConversationId,
        approved: ApprovedProposal,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        let _work = self.inner.hold_work();
        let ports = &self.inner.ports;
        let conversation = self.inner.require_conversation_here(conversation_id)?;
        let mut turns = ports.conversations.turns_of(conversation.id())?;
        if open_proposal_of(&turns).is_none() {
            return Err(ApplicationError::NoProposalOpen);
        }
        let mut project = self.inner.require_project(conversation.project_id())?;
        let team = ports.members.list_for_project(project.id())?;

        let (members, tasks) = Proposal::check_approval(
            approved
                .members
                .into_iter()
                .map(proposed_member_from)
                .collect::<Result<_, _>>()?,
            approved
                .tasks
                .into_iter()
                .map(proposed_task_from)
                .collect::<Result<_, _>>()?,
        )?;
        refuse_owners_who_are_not_there(&team, &members, &tasks)?;

        // Everything was checked. From here on nothing is refused, so the
        // members and the tasks are created together or not at all.
        let now = ports.clock.now();
        let mut colors: Vec<_> = team.iter().map(Member::color).collect();
        let mut by_name: HashMap<String, MemberId> = team
            .iter()
            .map(|member| (member.name().as_str().to_lowercase(), member.id()))
            .collect();
        let mut newcomers = Vec::new();
        for proposed in &members {
            let key = proposed.name.as_str().to_lowercase();
            if by_name.contains_key(&key) {
                // The team has this member already. It is not added again.
                continue;
            }
            let color = Member::color_for_newcomer(&colors);
            let mut member = Member::create(
                project.id(),
                proposed.name.clone(),
                MemberKind::Agent,
                proposed.responsibility.clone(),
                color,
                now,
            );
            member.set_profile(
                MemberProfile {
                    model: proposed.model.clone(),
                    effort: proposed.effort,
                },
                now,
            )?;
            colors.push(color);
            by_name.insert(key, member.id());
            newcomers.push(member);
        }

        let status = project.workflow().initial_status().clone();
        let mut last_rank = ports
            .tasks
            .list_for_project(project.id())?
            .iter()
            .filter(|task| task.status_id() == status.id())
            .map(Task::rank)
            .max();
        let mut created: Vec<Task> = Vec::new();
        for proposed in &tasks {
            let assignee = match &proposed.owner {
                None => None,
                Some(ProposedOwner::Manager) => Some(Assignee::Manager),
                Some(ProposedOwner::Member(name)) => by_name
                    .get(&name.as_str().to_lowercase())
                    .copied()
                    .map(Assignee::Member),
            };
            let rank = Rank::between(last_rank, None);
            last_rank = Some(rank);
            let number = project.allocate_task_number();
            created.push(Task::create(
                project.id(),
                number,
                &status,
                rank,
                TaskDraft {
                    title: proposed.title.clone(),
                    description: proposed.description.clone(),
                    priority: proposed.priority,
                    assignee,
                    parent_id: None,
                    estimate_minutes: proposed.estimate_minutes,
                    schedule: Schedule::unscheduled(),
                },
                now,
            ));
        }
        for (place, proposed) in tasks.iter().enumerate() {
            let blockers: Vec<_> = proposed
                .waits_for
                .iter()
                .filter_map(|&earlier| created.get(earlier).map(Task::id))
                .collect();
            if let Some(task) = created.get_mut(place) {
                for blocker in blockers {
                    task.block_on(blocker, now)?;
                }
            }
        }

        let proposing = turns.last_mut().ok_or(ApplicationError::NoProposalOpen)?;
        proposing.approve_proposal(members, tasks, now)?;
        ports.planning.save_approved_plan(&ApprovedPlan {
            project,
            members: newcomers,
            tasks: created,
            turn: proposing.clone(),
        })?;
        ports.listener.conversation_changed(&conversation, &turns);
        Ok(ConversationWithTurns {
            conversation,
            turns,
        })
    }

    /// Sends the Advisor a message of the manager's: a wish for a change of
    /// its proposal, or the start of another round. The caller holds the
    /// lock on the work.
    pub(super) fn reply_to_advisor(
        &self,
        conversation: Conversation,
        message: &str,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        let ports = &self.inner.ports;
        let turn = Turn::start(
            conversation.id(),
            message,
            TurnOrigin::Message,
            ports.clock.now(),
        )?;
        let earlier_turns = ports.conversations.turns_of(conversation.id())?;
        if open_interview_of(&earlier_turns).is_some() {
            return Err(ApplicationError::InterviewOpen);
        }
        self.inner
            .guard()
            .refuse_while_advisor_works(conversation.project_id())?;
        let project = self.inner.require_project(conversation.project_id())?;
        // The session of the Advisor does not see an approval happen. If it
        // is not told, it takes its proposal for open and proposes all of
        // it again.
        let approved_before = earlier_turns
            .last()
            .and_then(Turn::proposal)
            .is_some_and(|proposal| !proposal.is_open());
        let prompt = approved_before.then(|| message_after_approval(turn.request()));
        self.begin_turn(
            NewTurn {
                conversation,
                is_new_conversation: false,
                earlier_turns,
                answered_turn: None,
                turn,
                prompt,
            },
            Worker::Advisor,
            &project,
        )
    }
}

/// A member the manager approved, checked against the rules of members.
fn proposed_member_from(member: ApprovedMember) -> Result<ProposedMember, ApplicationError> {
    Ok(ProposedMember {
        name: MemberName::parse(&member.name)?,
        responsibility: member.responsibility,
        reason: member.reason,
        model: member
            .model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .map(ModelName::parse)
            .transpose()?,
        effort: member.effort,
    })
}

/// A task the manager approved, checked against the rules of tasks.
fn proposed_task_from(task: ApprovedTask) -> Result<ProposedTask, ApplicationError> {
    Ok(ProposedTask {
        title: TaskTitle::parse(&task.title)?,
        description: task.description,
        owner: match task.owner {
            None => None,
            Some(ApprovedOwner::Manager) => Some(ProposedOwner::Manager),
            Some(ApprovedOwner::Member(name)) => {
                Some(ProposedOwner::Member(MemberName::parse(&name)?))
            }
        },
        estimate_minutes: task.estimate_minutes,
        priority: task.priority,
        waits_for: task.waits_for,
    })
}

/// Refuses a task given to somebody who is neither on the team nor among
/// the members of the proposal, whatever the capitals.
fn refuse_owners_who_are_not_there(
    team: &[Member],
    members: &[ProposedMember],
    tasks: &[ProposedTask],
) -> Result<(), ApplicationError> {
    let known: HashSet<String> = team
        .iter()
        .map(|member| member.name().as_str().to_lowercase())
        .chain(
            members
                .iter()
                .map(|member| member.name.as_str().to_lowercase()),
        )
        .collect();
    for task in tasks {
        if let Some(ProposedOwner::Member(name)) = &task.owner
            && !known.contains(&name.as_str().to_lowercase())
        {
            return Err(ApplicationError::OwnerNotThere {
                title: task.title.to_string(),
                name: name.to_string(),
            });
        }
    }
    Ok(())
}

/// The answers to an interview as the Advisor reads them: every question
/// again, with the option that was chosen and what the manager added, or
/// with the manager's own words.
#[must_use]
pub fn answers_in_words(questions: &[InterviewQuestion], answers: &[InterviewAnswer]) -> String {
    let mut blocks = vec!["The manager answered your questions.".to_owned()];
    for (place, question) in questions.iter().enumerate() {
        let answer = answers.get(place);
        let note = answer.map_or("", |answer| answer.note.as_str());
        let chosen = answer
            .and_then(|answer| answer.option)
            .and_then(|option| question.options.get(option));
        let mut lines = vec![format!("{}. {}", place + 1, question.question)];
        match chosen {
            Some(option) => {
                lines.push(format!("   Answer: {}", option.label));
                if !note.is_empty() {
                    lines.push(format!("   The manager adds: {note}"));
                }
            }
            None => lines.push(format!("   Answer: {note}")),
        }
        blocks.push(lines.join("\n"));
    }
    blocks.join("\n\n")
}

/// A message of the manager's as the Advisor reads it when the manager
/// approved its proposal before writing: with the news of the approval, and
/// with what a proposal may hold from then on.
#[must_use]
pub fn message_after_approval(message: &str) -> String {
    format!(
        "The manager approved your proposal, as it was or with changes. What the project has \
         now is in your instructions: its members are on the team and its tasks exist. If what \
         the manager writes calls for more, propose only what is to be added: no member who is \
         on the team, no task that exists.\n\nThe manager writes:\n{message}"
    )
}

/// Writes the instructions that are added to the agent program's own for
/// the Advisor of a project: who it is, how it works, and what the project
/// has already. `team` and `tasks` are the members and the tasks of the
/// project, `reference_folders` the folders the Advisor may read besides
/// the work folder.
#[must_use]
pub fn instructions_for_advisor(
    project: &Project,
    team: &[Member],
    tasks: &[Task],
    reference_folders: &[PathBuf],
) -> String {
    let mut lines = vec![
        format!(
            "You are the Advisor of the project \"{}\" in Teamwright, a program in which a human \
             manager runs a project with a team of AI specialists. You are the manager's chief of \
             staff. You plan, the specialists do the work, and the manager decides.",
            project.name()
        ),
        String::new(),
        "Your job in this conversation is to turn what the manager wants into a team and a plan \
         that the manager can approve. You do not do the work of the project yourself, and you \
         write no files."
            .to_owned(),
        String::new(),
        "How you work:".to_owned(),
        "1. Read what the manager wrote. If the folder of the project holds material, read what \
         matters for the plan. You can read and search files and nothing else."
            .to_owned(),
        "2. Ask what you need to know with the tool interview. Ask only questions whose answer \
         changes the team or the plan: what is to be built, what \"done\" means, what is fixed \
         and what is free, what the manager does personally. One to five questions at a time, \
         and no more than two rounds. If the brief already answers a question, do not ask it. \
         Then end your turn; the answers come as your next message."
            .to_owned(),
        "3. Propose with the tool propose_plan: a summary of the project, the team, and the \
         tasks. Then end your turn with one or two sentences."
            .to_owned(),
    ];

    lines.push(String::new());
    lines.push("The material:".to_owned());
    lines.push("- The folder of the project is the folder you are in.".to_owned());
    if !reference_folders.is_empty() {
        // Each folder stands on a line of its own, because the name of a
        // folder may hold a comma.
        lines.push(
            "- Besides the folder of the project you may read the folders below. They hold \
             material the manager wants the team to know, and nothing in them may be changed. \
             Write their paths exactly as they stand here:"
                .to_owned(),
        );
        lines.extend(
            reference_folders
                .iter()
                .map(|folder| format!("    {}", folder.display())),
        );
    }
    lines.push(format!(
        "- Single files the manager handed over are copies in the folder {REFERENCES_FOLDER} \
         inside the folder of the project."
    ));

    if !team.is_empty() || !tasks.is_empty() {
        lines.push(String::new());
        lines.push(
            "What the project has already. Propose only what is to be added: members who are \
             not on the team yet and tasks that do not exist yet. Never list a member or a task \
             again that is there, also not one that you proposed earlier and the manager \
             approved. If nobody is to be added, the list of members is empty. Nothing that is \
             there is changed or removed. A new task may be given to a member who is there, by \
             the name below."
                .to_owned(),
        );
        if !team.is_empty() {
            lines.push("On the team already:".to_owned());
            lines.extend(team.iter().map(|member| {
                let kind = match member.kind() {
                    MemberKind::Agent => "AI specialist",
                    MemberKind::Human => "person",
                };
                let responsibility = member.responsibility().trim();
                if responsibility.is_empty() {
                    format!("- {} ({kind})", member.name())
                } else {
                    format!("- {} ({kind}): {responsibility}", member.name())
                }
            }));
        }
        if !tasks.is_empty() {
            lines.push("The tasks that exist:".to_owned());
            lines.extend(tasks.iter().map(|task| {
                let status = project.workflow().status(task.status_id()).map_or_else(
                    || "unknown status".to_owned(),
                    |status| status.name().to_string(),
                );
                let owner = match task.assignee() {
                    None => "nobody".to_owned(),
                    Some(Assignee::Manager) => "the manager".to_owned(),
                    Some(Assignee::Member(member_id)) => team
                        .iter()
                        .find(|member| member.id() == member_id)
                        .map_or_else(|| "nobody".to_owned(), |member| member.name().to_string()),
                };
                format!(
                    "- {}: {} ({status}, given to {owner})",
                    reference_of(project, task),
                    task.title()
                )
            }));
        }
    }

    lines.extend(
        [
            "",
            "The team:",
            "- Propose the smallest team that can do the job. Every member costs tokens and adds \
             coordination. One specialist for each discipline the project really needs, never \
             two for the same discipline. A project with one discipline gets one specialist.",
            "- A specialist is an AI. It reads and writes files in the folder of the project. It \
             cannot run programs, cannot use the internet, and has no hands.",
            "- Work that needs hands, money or an account is the manager's: ordering parts, \
             soldering, printing, measuring, publishing. Give those tasks to \"manager\".",
            "- Choose the model and the effort of each member: haiku for simple, well-described \
             work; sonnet for most work; opus for work that needs deep judgment. Low effort for \
             routine work. Leave both out when the default is right.",
            "",
            "The plan:",
            "- A task is a piece of work with a result that can be checked, which one member can \
             finish in one sitting.",
            "- Give each task a title that starts with a verb, a description that says what the \
             result is and how to tell that it is done, its owner, its size in hours of a \
             skilled person's work, and the tasks it waits for.",
            "- Where one discipline builds on the result of another, make that result a task of \
             its own with a name, for example \"Define the pin map\", and let the tasks that need \
             it wait for it.",
            "- Plan what the manager asked for and nothing more.",
            "",
            "Rules:",
            "- A choice that changes what is built, what it costs or when it is ready is the \
             manager's. Ask; do not assume.",
            "- Do not ask what a specialist will work out as part of its task. Such a choice \
             becomes a task of the plan, and if it is the manager's to make, a task for the \
             manager to approve it.",
            "- Do not write the word \"recommended\" into an option. Name the option with \
             recommended.",
            "- Write plainly: short sentences, and no word the manager would have to look up.",
        ]
        .map(str::to_owned),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests;
