//! The memory of a project: the decisions, facts and contracts the team
//! should not have to ask for twice ([decision 0036](../../../docs/decisions/0036-project-memory.md)).
//!
//! Entries come about in four ways:
//!
//! - When the manager answers a decision request of a specialist, the
//!   question and the answer become a decision ([`decision_entry`]),
//!   recorded by the work of the specialists without anybody writing it.
//! - When a handoff with an interface contract is published, the contract
//!   becomes a contract of the task, and replaces the contract an earlier
//!   handoff of the task left ([`contract_entries`]).
//! - A specialist records a fact with the tool `remember`, at most
//!   [`MOST_FACTS_PER_TURN`] in a turn.
//! - The manager adds facts, and changes and removes entries, through
//!   [`MemoryService`].
//!
//! A specialist reads memory twice: the brief of a task carries the entries
//! that most plainly belong to it ([`memory_in_words`]), and the tool
//! `search_memory` finds entries by a few words ([`search`]).

use std::sync::Arc;

use teamwright_domain::{
    Conversation, Handoff, Member, MemoryContent, MemoryEntry, MemoryEntryId, MemoryKind,
    MemoryOrigin, Project, ProjectId, SaidIn, SearchWords, Task, TaskId, Timestamp, Turn,
    memory::shorten,
};

use crate::agent_runtime::RememberedEntry;
use crate::error::ApplicationError;
use crate::ports::{Clock, MemoryRepository, ProjectRepository, TaskRepository};
use crate::work::reference_of;

/// The most facts a specialist records in one turn. More would let one
/// turn flood the memory that every later brief and search reads.
pub const MOST_FACTS_PER_TURN: usize = 3;

/// The most characters of the text of a fact a specialist records. A fact
/// is a sentence or two; longer results belong in a file and a handoff.
pub const MAX_FACT_LENGTH: usize = 2_000;

/// The most entries a search by a specialist answers with.
pub const MOST_FOUND: usize = 8;

/// The most characters of the text of one entry in the answer to a search.
pub const FOUND_TEXT_LENGTH: usize = 600;

/// The most characters the memory takes in the brief of a task.
pub const MEMORY_IN_BRIEF_LENGTH: usize = 3_000;

/// The most entries the brief of a task carries.
const MOST_IN_BRIEF: usize = 8;

/// The most characters of the text of one entry in a brief.
const BRIEF_TEXT_LENGTH: usize = 500;

/// Something a search in memory can look at: a title, a text and when it
/// was recorded.
pub trait Searchable {
    /// The short title.
    fn title(&self) -> &str;
    /// The text.
    fn text(&self) -> &str;
    /// When it was recorded.
    fn recorded_at(&self) -> Timestamp;
}

impl Searchable for MemoryEntry {
    fn title(&self) -> &str {
        Self::title(self)
    }

    fn text(&self) -> &str {
        Self::text(self)
    }

    fn recorded_at(&self) -> Timestamp {
        Self::recorded_at(self)
    }
}

impl Searchable for RememberedEntry {
    fn title(&self) -> &str {
        &self.title
    }

    fn text(&self) -> &str {
        &self.text
    }

    fn recorded_at(&self) -> Timestamp {
        self.recorded_at
    }
}

/// The entries that fit the words of a search, the newest first. Entries
/// recorded at the same moment keep the order they were given in.
#[must_use]
pub fn search<'entries, T: Searchable>(
    entries: &'entries [T],
    words: &SearchWords,
) -> Vec<&'entries T> {
    let mut found: Vec<&T> = entries
        .iter()
        .filter(|entry| words.fit(entry.title(), entry.text()))
        .collect();
    found.sort_by_key(|entry| std::cmp::Reverse(entry.recorded_at()));
    found
}

/// The current entries of a memory, the newest first.
#[must_use]
pub fn current_entries(entries: &[MemoryEntry]) -> Vec<&MemoryEntry> {
    let mut current: Vec<&MemoryEntry> =
        entries.iter().filter(|entry| entry.is_current()).collect();
    current.sort_by_key(|entry| std::cmp::Reverse((entry.recorded_at(), entry.id())));
    current
}

/// A kind of entry, as a word that starts a sentence.
#[must_use]
pub const fn kind_in_words(kind: MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Decision => "Decision",
        MemoryKind::Fact => "Fact",
        MemoryKind::Contract => "Contract",
    }
}

/// Where an entry came from, in words. `team` names the specialists; one
/// who has left the team is called a former teammate.
#[must_use]
pub fn source_in_words(origin: MemoryOrigin, team: &[Member]) -> String {
    let name = |said_in: SaidIn| {
        team.iter()
            .find(|member| member.id() == said_in.member_id)
            .map_or_else(
                || "a former teammate".to_owned(),
                |member| member.name().to_string(),
            )
    };
    match origin {
        MemoryOrigin::Manager => "written by the manager".to_owned(),
        MemoryOrigin::Specialist(said_in) => format!("remembered by {}", name(said_in)),
        MemoryOrigin::DecisionRequest(said_in) => {
            format!("decided by the manager, asked by {}", name(said_in))
        }
        MemoryOrigin::Handoff(said_in) => format!("handed over by {}", name(said_in)),
    }
}

/// The task of an entry in words, "SSN-3: Draw the schematic", if it has
/// one that still exists among `tasks`.
fn task_in_words(project: &Project, task_id: Option<TaskId>, tasks: &[Task]) -> Option<String> {
    let task = tasks.iter().find(|task| Some(task.id()) == task_id)?;
    Some(format!("{}: {}", reference_of(project, task), task.title()))
}

/// The current entries of a memory as a session that searches it reads
/// them, the newest first.
#[must_use]
pub fn remembered_entries(
    project: &Project,
    entries: &[MemoryEntry],
    tasks: &[Task],
    team: &[Member],
) -> Vec<RememberedEntry> {
    current_entries(entries)
        .into_iter()
        .map(|entry| RememberedEntry {
            kind: entry.kind(),
            title: entry.title().to_owned(),
            text: entry.text().to_owned(),
            source: source_in_words(entry.origin(), team),
            task: task_in_words(project, entry.task_id(), tasks),
            recorded_at: entry.recorded_at(),
        })
        .collect()
}

/// The decision a question and its answer become once the manager has
/// answered it: the question as the title, the question and the answer as
/// the text. `answer` is the answer in words, as the specialist is told
/// it. Nothing if the turn asked nothing or the conversation has no
/// specialist.
#[must_use]
pub fn decision_entry(
    conversation: &Conversation,
    asked: &Turn,
    answer: &str,
    now: Timestamp,
) -> Option<MemoryEntry> {
    let decision = asked.decision()?;
    let member_id = conversation.member_id()?;
    let title = MemoryContent::title_from(decision.question())?;
    let text = format!("Asked: {}\n\n{}", decision.question(), answer.trim());
    let content =
        MemoryContent::new(&title, &shorten(&text, MemoryContent::MAX_TEXT_LENGTH)).ok()?;
    Some(MemoryEntry::record(
        conversation.project_id(),
        MemoryKind::Decision,
        content,
        MemoryOrigin::DecisionRequest(SaidIn {
            member_id,
            conversation_id: conversation.id(),
            turn_id: asked.id(),
        }),
        conversation.task_id(),
        now,
    ))
}

/// What a handoff with an interface contract does to memory: the contract
/// becomes the contract of its task, and every current contract of the
/// same task is replaced by it, since the latest handoff of a task counts.
/// The first entry returned is the new one; the others are the replaced
/// ones, to be stored with it. Nothing if the handoff has no contract.
#[must_use]
pub fn contract_entries(
    project: &Project,
    task: &Task,
    handoff: &Handoff,
    existing: &[MemoryEntry],
) -> Vec<MemoryEntry> {
    let contract = handoff.content().contract();
    if contract.is_empty() {
        return Vec::new();
    }
    let title = shorten(
        &format!(
            "Interface of {}: {}",
            reference_of(project, task),
            task.title()
        ),
        MemoryContent::MAX_TITLE_LENGTH,
    );
    let Ok(content) = MemoryContent::new(&title, contract) else {
        return Vec::new();
    };
    let newer = MemoryEntry::record(
        handoff.project_id(),
        MemoryKind::Contract,
        content,
        MemoryOrigin::Handoff(SaidIn {
            member_id: handoff.author_id(),
            conversation_id: handoff.conversation_id(),
            turn_id: handoff.turn_id(),
        }),
        Some(task.id()),
        handoff.published_at(),
    );
    let mut stored = vec![newer.clone()];
    for older in existing {
        let replaced = older.kind() == MemoryKind::Contract
            && older.task_id() == Some(task.id())
            && older.is_current();
        if replaced {
            let mut older = older.clone();
            if older.replaced_with(&newer).is_ok() {
                stored.push(older);
            }
        }
    }
    stored
}

/// How many facts a specialist has remembered in a turn already.
#[must_use]
pub fn facts_remembered_in(entries: &[MemoryEntry], turn: &Turn) -> usize {
    entries
        .iter()
        .filter(|entry| {
            matches!(entry.origin(), MemoryOrigin::Specialist(said_in) if said_in.turn_id == turn.id())
        })
        .count()
}

/// What the memory of a project says to a specialist starting on a task,
/// as a part of its brief. Nothing if the memory is empty.
///
/// The entries that most plainly belong to the task come, within a budget
/// of [`MEMORY_IN_BRIEF_LENGTH`] characters and eight entries, each text
/// shortened: first the entries of the task itself; then those of the
/// tasks it builds on, except the contracts their handoffs left, which the
/// brief carries already; then every other contract; then the entries that
/// belong to no task, which hold for the whole project. Each group comes
/// the newest first. The brief always says how to find the rest.
#[must_use]
pub fn memory_in_words(
    project: &Project,
    task: &Task,
    blockers: &[&Task],
    entries: &[MemoryEntry],
    tasks: &[Task],
) -> Option<String> {
    let current = current_entries(entries);
    if current.is_empty() {
        return None;
    }
    let of_blocker = |entry: &MemoryEntry| {
        entry
            .task_id()
            .is_some_and(|task_id| blockers.iter().any(|blocker| blocker.id() == task_id))
    };
    let handed_over_already = |entry: &MemoryEntry| {
        of_blocker(entry) && matches!(entry.origin(), MemoryOrigin::Handoff(_))
    };
    let groups: [&dyn Fn(&MemoryEntry) -> bool; 4] = [
        &|entry| entry.task_id() == Some(task.id()),
        &|entry| of_blocker(entry) && !handed_over_already(entry),
        &|entry| entry.kind() == MemoryKind::Contract && !handed_over_already(entry),
        &|entry| entry.task_id().is_none(),
    ];

    let mut chosen: Vec<&MemoryEntry> = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    let mut used = 0;
    'groups: for belongs in groups {
        for entry in current.iter().copied().filter(|entry| belongs(entry)) {
            if chosen.iter().any(|other| other.id() == entry.id()) {
                continue;
            }
            if chosen.len() >= MOST_IN_BRIEF {
                break 'groups;
            }
            let part = entry_in_words(project, entry, tasks, BRIEF_TEXT_LENGTH);
            let length = part.chars().count();
            if used + length > MEMORY_IN_BRIEF_LENGTH {
                break 'groups;
            }
            used += length;
            chosen.push(entry);
            parts.push(part);
        }
    }

    let left = current.len() - chosen.len();
    let mut paragraphs = Vec::new();
    if parts.is_empty() {
        paragraphs.push(format!(
            "The project keeps a memory of decisions, facts and contracts ({} {}). Search it with \
             the tool search_memory before you ask the manager something that may have been \
             decided already.",
            current.len(),
            if current.len() == 1 {
                "entry"
            } else {
                "entries"
            }
        ));
    } else {
        paragraphs.push(
            "From the memory of the project, what bears on this task. Search the memory with the \
             tool search_memory before you ask the manager something that may have been decided \
             already."
                .to_owned(),
        );
        paragraphs.extend(parts);
        if left > 0 {
            paragraphs.push(format!(
                "{left} more {} in the memory; search for {}.",
                if left == 1 { "entry is" } else { "entries are" },
                if left == 1 { "it" } else { "them" }
            ));
        }
    }
    Some(paragraphs.join("\n\n"))
}

/// One entry as a brief shows it: its kind and title, its task and day,
/// and its text shortened to `length` characters.
fn entry_in_words(project: &Project, entry: &MemoryEntry, tasks: &[Task], length: usize) -> String {
    let mut about = Vec::new();
    if let Some(task) = task_in_words(project, entry.task_id(), tasks) {
        about.push(task);
    }
    about.push(entry.recorded_at().format("%Y-%m-%d").to_string());
    format!(
        "{}: {} ({})\n{}",
        kind_in_words(entry.kind()),
        entry.title(),
        about.join(", "),
        shorten(entry.text(), length)
    )
}

/// A fact the manager adds to the memory of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFact {
    /// The project.
    pub project_id: ProjectId,
    /// A short title of one line.
    pub title: String,
    /// What is true.
    pub text: String,
    /// The task it belongs to, if any.
    pub task_id: Option<TaskId>,
}

/// The use cases with which the manager reads and keeps the memory of a
/// project.
pub struct MemoryService {
    projects: Arc<dyn ProjectRepository>,
    tasks: Arc<dyn TaskRepository>,
    memory: Arc<dyn MemoryRepository>,
    clock: Arc<dyn Clock>,
}

impl MemoryService {
    /// Creates the service.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        memory: Arc<dyn MemoryRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            tasks,
            memory,
            clock,
        }
    }

    /// Every entry of the memory of a project, current and replaced, the
    /// newest first.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn memory_of_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<MemoryEntry>, ApplicationError> {
        self.require_project(project_id)?;
        let mut entries = self.memory.entries_of_project(project_id)?;
        entries.sort_by_key(|entry| std::cmp::Reverse((entry.recorded_at(), entry.id())));
        Ok(entries)
    }

    /// The current entries of a project that fit the words of a search,
    /// the newest first; all current entries if the search is blank. This
    /// is the rule a specialist's search follows too.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the search has too
    /// many words or characters, or if storage fails.
    pub fn search(
        &self,
        project_id: ProjectId,
        query: &str,
    ) -> Result<Vec<MemoryEntry>, ApplicationError> {
        self.require_project(project_id)?;
        let entries = self.memory.entries_of_project(project_id)?;
        let current: Vec<MemoryEntry> = current_entries(&entries).into_iter().cloned().collect();
        if query.trim().is_empty() {
            return Ok(current);
        }
        let words = SearchWords::parse(query)?;
        Ok(search(&current, &words).into_iter().cloned().collect())
    }

    /// Adds a fact the manager writes.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the task is not one
    /// of the project, if the title or text break a rule, or if storage
    /// fails.
    pub fn add_fact(&self, fact: NewFact) -> Result<MemoryEntry, ApplicationError> {
        self.require_project(fact.project_id)?;
        if let Some(task_id) = fact.task_id {
            let belongs = self
                .tasks
                .find(task_id)?
                .is_some_and(|task| task.project_id() == fact.project_id);
            if !belongs {
                return Err(ApplicationError::NotFound {
                    kind: "task",
                    id: task_id.to_string(),
                });
            }
        }
        let entry = MemoryEntry::record(
            fact.project_id,
            MemoryKind::Fact,
            MemoryContent::new(&fact.title, &fact.text)?,
            MemoryOrigin::Manager,
            fact.task_id,
            self.clock.now(),
        );
        self.memory.save_entries(std::slice::from_ref(&entry))?;
        Ok(entry)
    }

    /// Changes an entry: the manager's words become a newer entry of the
    /// same kind and task, which replaces it. The older entry stays and
    /// says so. Returns the newer entry.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such entry, if a newer one replaced
    /// it already, if the title or text break a rule, or if storage fails.
    pub fn change(
        &self,
        entry_id: MemoryEntryId,
        title: &str,
        text: &str,
    ) -> Result<MemoryEntry, ApplicationError> {
        let mut older = self.require_entry(entry_id)?;
        let newer = MemoryEntry::replace(
            &mut older,
            MemoryContent::new(title, text)?,
            MemoryOrigin::Manager,
            self.clock.now(),
        )?;
        self.memory.save_entries(&[newer.clone(), older])?;
        Ok(newer)
    }

    /// Removes an entry from memory.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such entry or if storage fails.
    pub fn remove(&self, entry_id: MemoryEntryId) -> Result<(), ApplicationError> {
        self.require_entry(entry_id)?;
        Ok(self.memory.delete_entry(entry_id)?)
    }

    fn require_project(&self, project_id: ProjectId) -> Result<Project, ApplicationError> {
        self.projects
            .find(project_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: project_id.to_string(),
            })
    }

    fn require_entry(&self, entry_id: MemoryEntryId) -> Result<MemoryEntry, ApplicationError> {
        self.memory
            .find_entry(entry_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "memory entry",
                id: entry_id.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone, Utc};
    use teamwright_domain::{
        AreaId, ConversationId, HandoffContent, MemberKind, MemberName, ProjectKey, ProjectName,
        Rank, TaskDraft, TaskTitle, TurnId, Workflow,
    };

    use super::*;
    use crate::in_memory::{
        InMemoryMemoryRepository, InMemoryProjectRepository, InMemoryTaskRepository, ManualClock,
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

    fn task(project: &Project, number: u32, title: &str) -> Task {
        Task::create(
            project.id(),
            AreaId::generate(),
            number,
            &project.workflow().statuses()[0],
            Rank::between(None, None),
            TaskDraft::titled(TaskTitle::parse(title).unwrap()),
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

    fn entry(
        project: &Project,
        kind: MemoryKind,
        title: &str,
        text: &str,
        task: Option<&Task>,
        minute: i64,
    ) -> MemoryEntry {
        MemoryEntry::record(
            project.id(),
            kind,
            MemoryContent::new(title, text).unwrap(),
            MemoryOrigin::Manager,
            task.map(Task::id),
            at(minute),
        )
    }

    #[test]
    fn a_search_finds_the_entries_that_fit_the_newest_first() {
        let project = project();
        let older = entry(
            &project,
            MemoryKind::Fact,
            "Display bus",
            "The display is on SPI2.",
            None,
            1,
        );
        let newer = entry(
            &project,
            MemoryKind::Contract,
            "Driver",
            "spi_write(bytes) on SPI2.",
            None,
            5,
        );
        let other = entry(
            &project,
            MemoryKind::Decision,
            "Power",
            "Battery only.",
            None,
            9,
        );
        let entries = [older.clone(), other, newer.clone()];

        let found = search(&entries, &SearchWords::parse("spi2").unwrap());

        assert_eq!(found, [&newer, &older]);
        assert!(search(&entries, &SearchWords::parse("solar").unwrap()).is_empty());
    }

    #[test]
    fn only_current_entries_are_searched_and_read_by_a_session() {
        let project = project();
        let pins = task(&project, 3, "Draw the pin map");
        let hardware = member(&project, "Hardware engineer");
        let mut older = MemoryEntry::record(
            project.id(),
            MemoryKind::Fact,
            MemoryContent::new("Enclosure", "Under 15 mm.").unwrap(),
            MemoryOrigin::Specialist(said_by(&hardware)),
            Some(pins.id()),
            at(1),
        );
        let newer = MemoryEntry::replace(
            &mut older,
            MemoryContent::new("Enclosure", "Under 12 mm.").unwrap(),
            MemoryOrigin::Manager,
            at(2),
        )
        .unwrap();
        let gone = MemoryEntry::record(
            project.id(),
            MemoryKind::Fact,
            MemoryContent::new("Clock", "8 MHz crystal.").unwrap(),
            MemoryOrigin::Specialist(said_by(&member(&project, "Tester"))),
            None,
            at(3),
        );
        let entries = [older.clone(), newer.clone(), gone];

        assert_eq!(current_entries(&entries).len(), 2);
        let read = remembered_entries(&project, &entries, &[pins], &[hardware]);
        assert_eq!(
            read.iter()
                .map(|entry| (
                    entry.title.as_str(),
                    entry.source.as_str(),
                    entry.task.as_deref()
                ))
                .collect::<Vec<_>>(),
            [
                ("Clock", "remembered by a former teammate", None),
                (
                    "Enclosure",
                    "written by the manager",
                    Some("SSN-3: Draw the pin map")
                ),
            ]
        );
        assert_eq!(read[1].text, "Under 12 mm.");
    }

    #[test]
    fn a_handoff_without_a_contract_leaves_memory_as_it_is() {
        let project = project();
        let pins = task(&project, 3, "Draw the pin map");
        let hardware = member(&project, "Hardware engineer");
        let handoff = Handoff::publish(
            project.id(),
            pins.id(),
            said_by(&hardware),
            HandoffContent::new("Done.", "", &[]).unwrap(),
            at(1),
        );

        assert!(contract_entries(&project, &pins, &handoff, &[]).is_empty());
    }

    #[test]
    fn the_brief_carries_what_belongs_to_the_task_first_within_its_budget() {
        let project = project();
        let drivers = task(&project, 4, "Write the drivers");
        let pins = task(&project, 3, "Draw the pin map");
        let case = task(&project, 5, "Design the case");
        let tasks = [drivers.clone(), pins.clone(), case.clone()];
        let entries = [
            entry(
                &project,
                MemoryKind::Fact,
                "Whole project",
                "Everything runs on 3.3 V.",
                None,
                1,
            ),
            entry(
                &project,
                MemoryKind::Contract,
                "Case screws",
                "M2 screws.",
                Some(&case),
                2,
            ),
            entry(
                &project,
                MemoryKind::Fact,
                "Pins",
                "Display on SPI2.",
                Some(&pins),
                3,
            ),
            entry(
                &project,
                MemoryKind::Decision,
                "Driver style",
                "Blocking calls.",
                Some(&drivers),
                4,
            ),
            entry(
                &project,
                MemoryKind::Fact,
                "Case colour",
                "Grey.",
                Some(&case),
                5,
            ),
        ];

        let words = memory_in_words(&project, &drivers, &[&pins], &entries, &tasks).unwrap();

        let titles: Vec<&str> = words
            .split("\n\n")
            .skip(1)
            .filter_map(|part| part.split(" (").next())
            .collect();
        assert_eq!(
            titles,
            [
                "Decision: Driver style",
                "Fact: Pins",
                "Contract: Case screws",
                "Fact: Whole project",
                "1 more entry is in the memory; search for it."
            ]
        );
        assert!(
            words.contains("Fact: Pins (SSN-3: Draw the pin map, 2026-11-21)\nDisplay on SPI2.")
        );
        assert_eq!(memory_in_words(&project, &drivers, &[], &[], &tasks), None);
    }

    #[test]
    fn memory_in_the_brief_never_takes_more_than_its_budget() {
        let project = project();
        let drivers = task(&project, 4, "Write the drivers");
        let long = "word ".repeat(400);
        let entries: Vec<MemoryEntry> = (0..20)
            .map(|minute| {
                entry(
                    &project,
                    MemoryKind::Fact,
                    "Long",
                    &long,
                    Some(&drivers),
                    minute,
                )
            })
            .collect();

        let words = memory_in_words(
            &project,
            &drivers,
            &[],
            &entries,
            std::slice::from_ref(&drivers),
        )
        .unwrap();

        // Five entries of about 540 characters fit in 3,000.
        assert_eq!(words.matches("Fact: Long").count(), 5);
        assert!(words.ends_with("15 more entries are in the memory; search for them."));

        let crowded: Vec<MemoryEntry> = (0..20)
            .map(|minute| entry(&project, MemoryKind::Fact, "Short", "Yes.", None, minute))
            .collect();
        let words = memory_in_words(&project, &drivers, &[], &crowded, &[]).unwrap();
        assert_eq!(words.matches("Fact: Short").count(), 8);
    }

    #[test]
    fn a_brief_points_to_memory_even_when_nothing_in_it_belongs_to_the_task() {
        let project = project();
        let drivers = task(&project, 4, "Write the drivers");
        let case = task(&project, 5, "Design the case");
        let entries = [entry(
            &project,
            MemoryKind::Fact,
            "Colour",
            "Grey.",
            Some(&case),
            1,
        )];

        assert_eq!(
            memory_in_words(&project, &drivers, &[], &entries, &[]).unwrap(),
            "The project keeps a memory of decisions, facts and contracts (1 entry). Search it \
             with the tool search_memory before you ask the manager something that may have \
             been decided already."
        );
    }

    struct Service {
        service: MemoryService,
        project: Project,
        task: Task,
    }

    fn service() -> Service {
        let project = project();
        let pins = task(&project, 3, "Draw the pin map");
        let projects = Arc::new(InMemoryProjectRepository::default());
        let tasks = Arc::new(InMemoryTaskRepository::default());
        crate::ports::ProjectRepository::save(projects.as_ref(), &project).unwrap();
        crate::ports::TaskRepository::save(tasks.as_ref(), &pins).unwrap();
        Service {
            service: MemoryService::new(
                projects,
                tasks,
                Arc::new(InMemoryMemoryRepository::default()),
                Arc::new(ManualClock::starting_at(at(10))),
            ),
            project,
            task: pins,
        }
    }

    #[test]
    fn the_manager_adds_a_fact_changes_it_and_removes_it() {
        let Service {
            service,
            project,
            task,
        } = service();
        let added = service
            .add_fact(NewFact {
                project_id: project.id(),
                title: "Enclosure".to_owned(),
                text: "Under 15 mm.".to_owned(),
                task_id: Some(task.id()),
            })
            .unwrap();
        assert_eq!(added.origin(), MemoryOrigin::Manager);
        assert_eq!(added.recorded_at(), at(10));

        let changed = service
            .change(added.id(), "Enclosure", "Under 12 mm.")
            .unwrap();

        let all = service.memory_of_project(project.id()).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(
            all.iter()
                .find(|entry| entry.id() == added.id())
                .unwrap()
                .replaced_by(),
            Some(changed.id())
        );
        assert_eq!(changed.task_id(), Some(task.id()));
        assert_eq!(
            service.search(project.id(), "12").unwrap(),
            std::slice::from_ref(&changed)
        );
        assert!(service.search(project.id(), "15").unwrap().is_empty());
        assert_eq!(
            service.search(project.id(), "  ").unwrap(),
            std::slice::from_ref(&changed)
        );
        assert!(matches!(
            service.change(added.id(), "Again", "No."),
            Err(ApplicationError::Rule(
                teamwright_domain::DomainError::MemoryEntryReplaced
            ))
        ));

        service.remove(changed.id()).unwrap();
        assert!(service.search(project.id(), "").unwrap().is_empty());
        assert!(service.remove(changed.id()).is_err());
    }

    #[test]
    fn a_fact_belongs_to_a_task_of_its_own_project_only() {
        let Service {
            service, project, ..
        } = service();
        let refused = service.add_fact(NewFact {
            project_id: project.id(),
            title: "Elsewhere".to_owned(),
            text: "Somewhere.".to_owned(),
            task_id: Some(TaskId::generate()),
        });
        assert!(matches!(
            refused,
            Err(ApplicationError::NotFound { kind: "task", .. })
        ));
        assert!(service.memory_of_project(ProjectId::generate()).is_err());
        assert!(matches!(
            service.search(project.id(), "a b c d e f g h i j k l m"),
            Err(ApplicationError::Rule(_))
        ));
    }
}
