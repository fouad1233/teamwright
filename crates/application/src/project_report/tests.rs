//! The report of a project in memory: what each of its parts says, and
//! where the stored data cannot say something, that it says so.

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone};
use teamwright_domain::{
    ComputerId, Conversation, DecisionOption, DecisionRequest, HandoffContent, Interview,
    InterviewAnswer, InterviewQuestion, InterviewSnapshot, MemberKind, Proposal, ProposalSnapshot,
    SaidIn, StatusCategory, StatusId, TokenCounts, Turn, TurnOrigin,
};

use super::*;
use crate::areas::AreaService;
use crate::in_memory::{
    InMemoryAreaRepository, InMemoryConversationRepository, InMemoryMemberRepository,
    InMemoryPlacesRepository, InMemoryProjectRepository, InMemorySprintRepository,
    InMemoryTaskRepository, InMemoryTeamworkRepository, InMemoryViewRepository, ManualClock,
};
use crate::projects::{NewProject, ProjectService};
use crate::sprints::{NewSprint, SprintService, SprintStart};
use crate::tasks::{NewTask, TaskService};
use crate::team::{NewMember, TeamService};

/// A project, the services that fill it, and the use case under test, on
/// shared storage. The clock shows 13 November 2026 at nine.
struct Fixture {
    tasks: TaskService,
    sprints: SprintService,
    team: TeamService,
    areas: AreaService,
    conversations: Arc<InMemoryConversationRepository>,
    teamwork: Arc<InMemoryTeamworkRepository>,
    clock: Arc<ManualClock>,
    report: ProjectReportService,
    project: Project,
}

fn moment() -> Timestamp {
    chrono::Utc.with_ymd_and_hms(2026, 11, 13, 9, 0, 0).unwrap()
}

/// A day of November 2026.
fn day(day_of_month: u32) -> CalendarDate {
    CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
}

fn fixture() -> Fixture {
    let projects_store = Arc::new(InMemoryProjectRepository::default());
    let tasks_store = Arc::new(InMemoryTaskRepository::default());
    let members_store = Arc::new(InMemoryMemberRepository::default());
    let conversations = Arc::new(InMemoryConversationRepository::default());
    let sprints_store = Arc::new(InMemorySprintRepository::new(tasks_store.clone()));
    let areas_store = Arc::new(InMemoryAreaRepository::default());
    let views_store = Arc::new(InMemoryViewRepository::default());
    let teamwork = Arc::new(InMemoryTeamworkRepository::default());
    let clock = Arc::new(ManualClock::starting_at(moment()));
    let projects = ProjectService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations.clone(),
        Arc::new(InMemoryPlacesRepository::default()),
        sprints_store.clone(),
        views_store.clone(),
        areas_store.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations.clone(),
        sprints_store.clone(),
        areas_store.clone(),
        clock.clone(),
    );
    let sprints = SprintService::new(
        projects_store.clone(),
        tasks_store.clone(),
        sprints_store.clone(),
        clock.clone(),
    );
    let team = TeamService::new(
        projects_store.clone(),
        members_store.clone(),
        tasks_store.clone(),
        conversations.clone(),
        views_store,
        clock.clone(),
    );
    let areas = AreaService::new(
        projects_store.clone(),
        tasks_store.clone(),
        areas_store.clone(),
        clock.clone(),
    );
    let report = ProjectReportService::new(ProjectReportPorts {
        projects: projects_store,
        tasks: tasks_store,
        sprints: sprints_store,
        areas: areas_store,
        members: members_store,
        conversations: conversations.clone(),
        teamwork: teamwork.clone(),
        clock: clock.clone(),
    });
    let project = projects
        .create(NewProject {
            name: "Smart home hub".to_owned(),
            key: Some("SH".to_owned()),
            description: "A hub that ties the lights together.".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    Fixture {
        tasks,
        sprints,
        team,
        areas,
        conversations,
        teamwork,
        clock,
        report,
        project,
    }
}

impl Fixture {
    fn task(&self, title: &str) -> Task {
        self.tasks
            .create(NewTask::titled(self.project.id(), title))
            .unwrap()
    }

    fn status_of(&self, category: StatusCategory) -> StatusId {
        self.project.workflow().first_in(category).unwrap().id()
    }

    fn finish(&self, task: &Task) {
        self.tasks
            .move_to(task.id(), self.status_of(StatusCategory::Done), 0)
            .unwrap();
    }

    fn due(&self, task: &Task, due_on: CalendarDate) {
        self.tasks
            .reschedule(task.id(), None, Some(due_on))
            .unwrap();
    }

    fn member(&self, name: &str, kind: MemberKind) -> Member {
        self.team
            .add(NewMember {
                project_id: self.project.id(),
                name: name.to_owned(),
                kind,
                responsibility: String::new(),
                model: None,
                effort: None,
            })
            .unwrap()
    }

    /// A conversation with a member about a task, or about nothing.
    fn conversation(&self, member: &Member, task: Option<&Task>) -> Conversation {
        let conversation = Conversation::start(
            self.project.id(),
            member.id(),
            task.map(Task::id),
            moment(),
            ComputerId::generate(),
        );
        self.conversations.save_conversation(&conversation).unwrap();
        conversation
    }

    /// A turn that ran `minutes` on a model, used `tokens`, and was then
    /// changed by `then` before it was stored.
    fn turn(
        &self,
        conversation: &Conversation,
        minutes: i64,
        model: Option<&str>,
        tokens: u64,
        then: impl FnOnce(&mut Turn),
    ) -> Turn {
        let mut turn =
            Turn::start(conversation.id(), "Begin", TurnOrigin::Message, moment()).unwrap();
        if let Some(model) = model {
            turn.record_model(model.to_owned()).unwrap();
        }
        then(&mut turn);
        turn.finish(
            "Done",
            TokenCounts {
                input: tokens,
                ..TokenCounts::default()
            },
            moment() + TimeDelta::minutes(minutes),
        )
        .unwrap();
        self.conversations.save_turn(&turn).unwrap();
        turn
    }

    fn report(&self) -> ProjectReport {
        self.report.report_of(self.project.id()).unwrap()
    }
}

fn decision(question: &str) -> DecisionRequest {
    DecisionRequest::new(
        question,
        "",
        vec![
            DecisionOption {
                label: "One 18650 cell".to_owned(),
                consequence: String::new(),
            },
            DecisionOption {
                label: "Two AA cells".to_owned(),
                consequence: String::new(),
            },
        ],
        None,
    )
    .unwrap()
}

#[test]
fn a_project_without_tasks_is_reported_so_far_and_says_it_has_none() {
    let fixture = fixture();

    let report = fixture.report();

    assert!(!report.is_complete);
    assert_eq!(report.title, TITLE_SO_FAR);
    assert_eq!(report.name, "Smart home hub");
    assert_eq!(report.made_on, day(13));
    assert_eq!(
        report.summary,
        ["The project was begun on 13 November 2026. It has no tasks yet."]
    );
    assert_eq!(
        report.plan.sentences,
        ["No task is finished yet, so nothing can be compared."]
    );
    assert!(report.finished.is_empty());
    assert!(report.open.is_empty());
    assert!(report.brief.is_none());
    assert!(report.sprints.is_empty());
    assert!(report.decisions.is_empty());
    assert!(report.handoffs.is_empty());
    assert_eq!(
        report.usage,
        ReportUsage {
            total: UsageLine {
                name: "Everyone".to_owned(),
                ..UsageLine::default()
            },
            ..ReportUsage::default()
        }
    );
}

#[test]
fn a_project_whose_every_task_is_finished_is_complete() {
    let fixture = fixture();
    let first = fixture.task("Draw the schematic");
    let second = fixture.task("Route the board");
    fixture.finish(&first);

    assert!(!fixture.report().is_complete);
    assert_eq!(
        fixture.report().summary[1],
        "1 of its 2 tasks is finished, and 1 is still open."
    );

    fixture.clock.advance(TimeDelta::days(2));
    fixture.finish(&second);
    let report = fixture.report();

    assert!(report.is_complete);
    assert_eq!(report.title, TITLE_OF_COMPLETE);
    assert_eq!(
        report.summary,
        [
            "The project was begun on 13 November 2026, and its last task was finished on 15 November 2026.",
            "All 2 of its tasks are finished.",
        ]
    );
    assert!(report.open.is_empty());
    assert_eq!(
        report
            .finished
            .iter()
            .map(|task| (task.reference.as_str(), task.finished_on))
            .collect::<Vec<_>>(),
        [("SH-1", Some(day(13))), ("SH-2", Some(day(15)))]
    );
}

#[test]
fn due_dates_are_met_missed_or_late_and_counted_in_the_plan() {
    let fixture = fixture();
    let on_time = fixture.task("On time");
    let after = fixture.task("Finished after");
    let late = fixture.task("Still open and late");
    let coming = fixture.task("Due next week");
    let undated = fixture.task("No date");
    fixture.due(&on_time, day(13));
    fixture.due(&after, day(10));
    fixture.due(&late, day(11));
    fixture.due(&coming, day(20));
    fixture.finish(&on_time);
    fixture.finish(&after);
    fixture.finish(&undated);

    let report = fixture.report();
    let due_of = |reference: &str| {
        report
            .finished
            .iter()
            .chain(&report.open)
            .find(|task| task.reference == reference)
            .unwrap()
            .due
    };

    assert_eq!(due_of("SH-1"), DueOutcome::Met);
    assert_eq!(due_of("SH-2"), DueOutcome::Missed { days: 3 });
    assert_eq!(due_of("SH-3"), DueOutcome::Late { days: 2 });
    assert_eq!(due_of("SH-4"), DueOutcome::Coming);
    assert_eq!(due_of("SH-5"), DueOutcome::NoDueDate);
    assert_eq!(
        (
            report.plan.with_due_date,
            report.plan.met,
            report.plan.missed,
            report.plan.late
        ),
        (2, 1, 1, 1)
    );
    assert!(report.plan.sentences.contains(
        &"Of the 2 finished tasks with a due date, 1 finished by it and 1 after it.".to_owned()
    ));
    assert!(
        report
            .plan
            .sentences
            .contains(&"1 open task is past its due date.".to_owned())
    );
}

#[test]
fn estimates_are_set_against_how_long_the_turns_of_the_specialists_ran() {
    let fixture = fixture();
    let ada = fixture.member("Ada", MemberKind::Agent);
    let measured = fixture.task("Measured");
    let unmeasured = fixture.task("Estimated only");
    fixture.tasks.estimate(measured.id(), Some(240)).unwrap();
    fixture.tasks.estimate(unmeasured.id(), Some(60)).unwrap();
    fixture
        .tasks
        .assign(measured.id(), Some(Assignee::Member(ada.id())))
        .unwrap();
    fixture
        .tasks
        .assign(unmeasured.id(), Some(Assignee::Member(ada.id())))
        .unwrap();
    let conversation = fixture.conversation(&ada, Some(&measured));
    fixture.turn(&conversation, 50, None, 0, |_| {});
    fixture.turn(&conversation, 40, None, 0, |_| {});
    fixture.finish(&measured);
    fixture.finish(&unmeasured);

    let report = fixture.report();

    assert_eq!(report.finished[0].worked_minutes, Some(90));
    assert_eq!(report.finished[1].worked_minutes, None);
    assert_eq!(
        report.plan.sentences,
        [
            "All 2 finished tasks had an estimate: 5 hours together.",
            "The specialists' turns on 1 finished task ran 1.5 hours together.",
            "On the 1 task with both, the estimates came to 4 hours and the turns ran 1.5 hours. An estimate is the time a skilled person would need; the turns are how long the specialists worked.",
            "No finished task had a due date.",
        ]
    );
}

#[test]
fn work_done_by_people_is_said_to_be_counted_and_not_measured() {
    let fixture = fixture();
    let grace = fixture.member("Grace", MemberKind::Human);
    let task = fixture.task("Solder the board");
    fixture
        .tasks
        .assign(task.id(), Some(Assignee::Member(grace.id())))
        .unwrap();
    fixture.finish(&task);

    let report = fixture.report();

    assert!(report.plan.sentences.contains(
        &"Teamwright does not time the work of people, so their tasks are counted and not measured."
            .to_owned()
    ));
    assert!(report.plan.sentences.contains(
        &"No specialist had a turn on a finished task, so no time was measured.".to_owned()
    ));
}

#[test]
fn the_work_is_shared_by_area_only_when_there_is_more_than_one() {
    let fixture = fixture();
    let first = fixture.task("Draw the schematic");
    fixture.finish(&first);

    assert!(fixture.report().by_area.is_empty());
    assert_eq!(fixture.report().finished[0].area, None);

    let enclosure = fixture
        .areas
        .add(fixture.project.id(), "Enclosure")
        .unwrap();
    fixture
        .tasks
        .create(NewTask {
            area_id: Some(enclosure.id()),
            ..NewTask::titled(fixture.project.id(), "Print the case")
        })
        .unwrap();
    let report = fixture.report();

    assert_eq!(report.by_area.len(), 2);
    assert_eq!(
        report.by_area[1],
        WorkShare {
            name: "Enclosure".to_owned(),
            finished: 0,
            open: 1,
        }
    );
    assert_eq!(report.open[0].area.as_deref(), Some("Enclosure"));
}

#[test]
fn the_work_is_shared_by_members_then_the_manager_then_nobody() {
    let fixture = fixture();
    let ada = fixture.member("Ada", MemberKind::Agent);
    let _idle = fixture.member("Idle", MemberKind::Agent);
    let mine = fixture.task("Mine");
    let ada_s = fixture.task("Ada's");
    let _nobody_s = fixture.task("Nobody's");
    fixture
        .tasks
        .assign(mine.id(), Some(Assignee::Manager))
        .unwrap();
    fixture
        .tasks
        .assign(ada_s.id(), Some(Assignee::Member(ada.id())))
        .unwrap();
    fixture.finish(&ada_s);

    let report = fixture.report();

    assert_eq!(
        report
            .by_holder
            .iter()
            .map(|share| (share.name.as_str(), share.finished, share.open))
            .collect::<Vec<_>>(),
        [("Ada", 1, 0), ("The manager", 0, 1), ("Nobody", 0, 1)]
    );
    assert_eq!(report.finished[0].holder, "Ada");
}

#[test]
fn the_decisions_answered_are_listed_in_the_order_they_were_answered() {
    let fixture = fixture();
    let ada = fixture.member("Ada", MemberKind::Agent);
    let task = fixture.task("Choose the battery");
    let conversation = fixture.conversation(&ada, Some(&task));
    let mut asked = fixture.turn(&conversation, 1, None, 0, |turn| {
        turn.ask(decision("Which battery?")).unwrap();
    });
    asked
        .answer_decision(
            Some(0),
            "It must last a year.",
            moment() + TimeDelta::days(1),
        )
        .unwrap();
    fixture.conversations.save_turn(&asked).unwrap();
    // A question that waits is not a decision yet.
    fixture.turn(&conversation, 1, None, 0, |turn| {
        turn.ask(decision("Which charger?")).unwrap();
    });
    let advisor = Conversation::start_with_advisor(
        fixture.project.id(),
        None,
        None,
        moment(),
        ComputerId::generate(),
    );
    fixture.conversations.save_conversation(&advisor).unwrap();
    fixture.turn(&advisor, 1, None, 0, |turn| {
        turn.ask_interview(Interview::restore(InterviewSnapshot {
            questions: vec![InterviewQuestion {
                question: "Who will use it?".to_owned(),
                reason: String::new(),
                options: Vec::new(),
                recommended: None,
            }],
            answers: Some(vec![InterviewAnswer {
                option: None,
                note: "Families.".to_owned(),
            }]),
            answered_at: Some(moment()),
        }))
        .unwrap();
    });

    let report = fixture.report();

    assert_eq!(
        report.decisions,
        [
            ReportDecision {
                asked_by: "The Advisor".to_owned(),
                task: None,
                question: "Who will use it?".to_owned(),
                chosen: None,
                note: "Families.".to_owned(),
                answered_on: day(13),
            },
            ReportDecision {
                asked_by: "Ada".to_owned(),
                task: Some("SH-1".to_owned()),
                question: "Which battery?".to_owned(),
                chosen: Some("One 18650 cell".to_owned()),
                note: "It must last a year.".to_owned(),
                answered_on: day(14),
            },
        ]
    );
    assert_eq!(report.waiting, 1);
    assert!(
        report
            .summary
            .contains(&"1 question still waits for the manager.".to_owned())
    );
}

#[test]
fn the_brief_is_the_summary_of_the_latest_plan_the_manager_approved() {
    let fixture = fixture();
    let advisor = Conversation::start_with_advisor(
        fixture.project.id(),
        None,
        None,
        moment(),
        ComputerId::generate(),
    );
    fixture.conversations.save_conversation(&advisor).unwrap();
    let proposal = |summary: &str, approved_at: Option<Timestamp>| {
        Proposal::restore(ProposalSnapshot {
            summary: summary.to_owned(),
            members: Vec::new(),
            tasks: Vec::new(),
            approved_at,
        })
    };
    fixture.turn(&advisor, 1, None, 0, |turn| {
        turn.propose(proposal("A first plan.", Some(moment())))
            .unwrap();
    });
    fixture.turn(&advisor, 1, None, 0, |turn| {
        turn.propose(proposal(
            "A second plan.",
            Some(moment() + TimeDelta::days(2)),
        ))
        .unwrap();
    });
    fixture.turn(&advisor, 1, None, 0, |turn| {
        turn.propose(proposal("A plan nobody approved.", None))
            .unwrap();
    });

    assert_eq!(
        fixture.report().brief,
        Some(ReportBrief {
            summary: "A second plan.".to_owned(),
            approved_on: day(15),
        })
    );
}

#[test]
fn the_results_handed_over_are_the_latest_handoff_of_every_task() {
    let fixture = fixture();
    let ada = fixture.member("Ada", MemberKind::Agent);
    let task = fixture.task("Choose the microcontroller");
    let _other = fixture.task("Nothing handed over");
    let conversation = fixture.conversation(&ada, Some(&task));
    let turn = fixture.turn(&conversation, 1, None, 0, |_| {});
    let said_in = SaidIn {
        member_id: ada.id(),
        conversation_id: conversation.id(),
        turn_id: turn.id(),
    };
    for (summary, days) in [("A first look.", 0), ("The ESP32-C3 it is.", 1)] {
        fixture
            .teamwork
            .save_handoff(&Handoff::publish(
                fixture.project.id(),
                task.id(),
                said_in,
                HandoffContent::new(summary, "GPIO4 = SDA", &["docs/mcu.md".to_owned()]).unwrap(),
                moment() + TimeDelta::days(days),
            ))
            .unwrap();
    }

    assert_eq!(
        fixture.report().handoffs,
        [ReportHandoff {
            reference: "SH-1".to_owned(),
            title: "Choose the microcontroller".to_owned(),
            author: "Ada".to_owned(),
            published_on: day(14),
            summary: "The ESP32-C3 it is.".to_owned(),
            contract: "GPIO4 = SDA".to_owned(),
            files: vec!["docs/mcu.md".to_owned()],
        }]
    );
}

#[test]
fn the_usage_is_counted_by_who_and_by_model() {
    let fixture = fixture();
    let ada = fixture.member("Ada", MemberKind::Agent);
    let bob = fixture.member("Bob", MemberKind::Agent);
    let with_ada = fixture.conversation(&ada, None);
    let with_bob = fixture.conversation(&bob, None);
    fixture.turn(&with_ada, 1, Some("opus"), 300, |_| {});
    fixture.turn(&with_bob, 1, Some("sonnet"), 1000, |_| {});
    fixture.turn(&with_bob, 1, Some("opus"), 200, |_| {});

    let report = fixture.report();
    let lines = |lines: &[UsageLine]| {
        lines
            .iter()
            .map(|line| (line.name.clone(), line.tokens, line.turns))
            .collect::<Vec<_>>()
    };

    assert_eq!(
        lines(&report.usage.by_who),
        [("Bob".to_owned(), 1200, 2), ("Ada".to_owned(), 300, 1)]
    );
    assert_eq!(
        lines(&report.usage.by_model),
        [("sonnet".to_owned(), 1000, 1), ("opus".to_owned(), 500, 2)]
    );
    assert_eq!(
        (report.usage.total.tokens, report.usage.total.turns),
        (1500, 3)
    );
    assert!(
        report
            .summary
            .contains(&"The team took 3 turns and used 1,500 tokens.".to_owned())
    );
}

#[test]
fn a_sprint_says_what_it_took_on_and_what_it_finished() {
    let fixture = fixture();
    let first = fixture.task("Draw the schematic");
    let second = fixture.task("Route the board");
    let sprint = fixture
        .sprints
        .create(NewSprint::in_project(fixture.project.id()))
        .unwrap();
    fixture.tasks.plan(first.id(), Some(sprint.id())).unwrap();
    fixture.tasks.plan(second.id(), Some(sprint.id())).unwrap();
    fixture.finish(&first);
    fixture
        .sprints
        .start(
            sprint.id(),
            SprintStart {
                name: "Sprint 1".to_owned(),
                goal: "A board".to_owned(),
                start_on: day(10),
                end_on: day(17),
            },
        )
        .unwrap();

    let report = fixture.report();

    assert_eq!(
        report.sprints,
        [SprintOutcome {
            name: "Sprint 1".to_owned(),
            goal: "A board".to_owned(),
            state: SprintState::Active,
            start_on: Some(day(10)),
            end_on: Some(day(17)),
            unit: SprintUnit::Tasks,
            committed: 2,
            completed: 1,
        }]
    );
}

#[test]
fn durations_and_numbers_are_said_plainly() {
    assert_eq!(duration_in_words(1), "1 minute");
    assert_eq!(duration_in_words(45), "45 minutes");
    assert_eq!(duration_in_words(60), "1 hour");
    assert_eq!(duration_in_words(90), "1.5 hours");
    assert_eq!(duration_in_words(125), "2.1 hours");
    assert_eq!(number_in_words(0), "0");
    assert_eq!(number_in_words(999), "999");
    assert_eq!(number_in_words(1000), "1,000");
    assert_eq!(number_in_words(1_234_567), "1,234,567");
    assert_eq!(date_in_words(day(1)), "1 November 2026");
}

#[test]
fn a_project_that_does_not_exist_is_not_found() {
    let fixture = fixture();

    assert!(matches!(
        fixture.report.report_of(ProjectId::generate()),
        Err(ApplicationError::NotFound {
            kind: "project",
            ..
        })
    ));
}
