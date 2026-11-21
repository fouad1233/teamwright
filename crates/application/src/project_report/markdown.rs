//! The report of a project written as Markdown, to be kept as a file in
//! the work folder of the project, next to the work it reports on.
//!
//! The function is pure: it turns the report into text and nothing else.
//! A part with nothing to say is left out, never written as an empty
//! heading. The words are those the report carries, so that the file and
//! the page on the screen say the same.

use std::fmt::Write as _;

use teamwright_domain::{CalendarDate, SprintState, SprintUnit};

use super::{
    DueOutcome, ProjectReport, ReportDecision, ReportHandoff, ReportTask, SprintOutcome, UsageLine,
    WorkShare, date_in_words, number_in_words,
};

/// The report as a Markdown document.
#[must_use]
pub fn report_in_markdown(report: &ProjectReport) -> String {
    let mut text = String::new();
    line(&mut text, &format!("# {}", one_line(&report.name)));
    blank(&mut text);
    line(
        &mut text,
        &format!(
            "{}, made on {}. The key of its tasks is {}.",
            report.title,
            date_in_words(report.made_on),
            report.key
        ),
    );
    if !report.description.is_empty() {
        blank(&mut text);
        line(&mut text, &report.description);
    }

    section(&mut text, "Summary");
    line(&mut text, &report.summary.join(" "));

    if let Some(brief) = &report.brief {
        section(&mut text, "What was to be built");
        line(&mut text, &brief.summary);
        blank(&mut text);
        line(
            &mut text,
            &format!(
                "*The plan of the Advisor, as the manager approved it on {}.*",
                date_in_words(brief.approved_on)
            ),
        );
    }

    section(&mut text, "What was done");
    if report.finished.is_empty() {
        line(&mut text, "Nothing is finished yet.");
    } else {
        if !report.by_area.is_empty() {
            subsection(&mut text, "By area");
            shares(&mut text, "Area", &report.by_area);
        }
        if !report.by_holder.is_empty() {
            subsection(&mut text, "By who held the work");
            shares(&mut text, "Who", &report.by_holder);
        }
        subsection(&mut text, "The finished tasks");
        finished_tasks(&mut text, &report.finished);
    }

    section(&mut text, "Planned against what happened");
    for sentence in &report.plan.sentences {
        line(&mut text, &format!("- {sentence}"));
    }

    if !report.sprints.is_empty() {
        section(&mut text, "Sprints");
        sprints(&mut text, &report.sprints);
    }

    if report.usage.total.turns > 0 {
        section(&mut text, "What the team used");
        usage(
            &mut text,
            "Who",
            &report.usage.by_who,
            Some(&report.usage.total),
        );
        blank(&mut text);
        usage(&mut text, "Model", &report.usage.by_model, None);
    }

    if !report.decisions.is_empty() {
        section(&mut text, "Decisions of the manager");
        for decision in &report.decisions {
            self::decision(&mut text, decision);
        }
    }

    if !report.handoffs.is_empty() {
        section(&mut text, "Results handed over");
        for handoff in &report.handoffs {
            self::handoff(&mut text, handoff);
        }
    }

    if !report.open.is_empty() || report.waiting > 0 {
        section(&mut text, "Still open");
        if !report.open.is_empty() {
            open_tasks(&mut text, &report.open);
        }
        if report.waiting > 0 {
            if !report.open.is_empty() {
                blank(&mut text);
            }
            line(
                &mut text,
                &if report.waiting == 1 {
                    "1 question waits for the manager's answer.".to_owned()
                } else {
                    format!(
                        "{} questions wait for the manager's answer.",
                        report.waiting
                    )
                },
            );
        }
    }

    blank(&mut text);
    line(&mut text, "---");
    blank(&mut text);
    line(
        &mut text,
        &format!(
            "Made with Teamwright on {}, from what the project has stored.",
            date_in_words(report.made_on)
        ),
    );
    text
}

/// The characters a file name may not hold on Windows, the strictest of the
/// systems, and the ones that would make it a hidden file elsewhere.
const NOT_IN_FILE_NAMES: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// The name a saved report gets: the name of the project, the word
/// "report" and the day it was made, so that a report saved later does not
/// replace an earlier one: `Smart sensor node report 2026-11-21.md`.
#[must_use]
pub fn report_file_name(project_name: &str, made_on: CalendarDate) -> String {
    let name: String = project_name
        .chars()
        .map(|character| {
            if NOT_IN_FILE_NAMES.contains(&character) || character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    let name = one_line(name.trim_start_matches('.'));
    let name = if name.is_empty() { "Project" } else { &name };
    format!("{name} report {}.md", made_on.format("%Y-%m-%d"))
}

/// What a task's due date says in a table.
#[must_use]
pub fn due_in_words(due: DueOutcome, due_on: Option<CalendarDate>) -> String {
    match due {
        DueOutcome::NoDueDate => String::new(),
        DueOutcome::Met => "On time".to_owned(),
        DueOutcome::Missed { days } | DueOutcome::Late { days } => days_late(days),
        DueOutcome::Coming => due_on.map(date_in_words).unwrap_or_default(),
    }
}

fn days_late(days: u32) -> String {
    if days == 1 {
        "1 day late".to_owned()
    } else {
        format!("{days} days late")
    }
}

fn line(text: &mut String, words: &str) {
    text.push_str(words);
    text.push('\n');
}

fn blank(text: &mut String) {
    text.push('\n');
}

fn section(text: &mut String, heading: &str) {
    blank(text);
    line(text, &format!("## {heading}"));
    blank(text);
}

fn subsection(text: &mut String, heading: &str) {
    if !text.ends_with("\n\n") {
        blank(text);
    }
    line(text, &format!("### {heading}"));
    blank(text);
}

/// Text on one line, for a heading.
fn one_line(words: &str) -> String {
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Text that is safe in a cell of a table: on one line, its bars escaped.
fn cell(words: &str) -> String {
    one_line(words).replace('|', "\\|")
}

/// A table: a header row, the line under it, and the rows. A column whose
/// name ends in a colon is a column of figures, set to the right; the
/// colon is not written.
fn table(text: &mut String, columns: &[&str], rows: &[Vec<String>]) {
    let names: Vec<&str> = columns
        .iter()
        .map(|column| column.trim_end_matches(':'))
        .collect();
    line(text, &format!("| {} |", names.join(" | ")));
    let rules: Vec<&str> = columns
        .iter()
        .map(|column| if column.ends_with(':') { "---:" } else { "---" })
        .collect();
    line(text, &format!("| {} |", rules.join(" | ")));
    for row in rows {
        let cells: Vec<String> = row.iter().map(|value| cell(value)).collect();
        line(text, &format!("| {} |", cells.join(" | ")));
    }
}

fn shares(text: &mut String, name: &str, shares: &[WorkShare]) {
    table(
        text,
        &[name, "Finished:", "Open:"],
        &shares
            .iter()
            .map(|share| {
                vec![
                    share.name.clone(),
                    share.finished.to_string(),
                    share.open.to_string(),
                ]
            })
            .collect::<Vec<_>>(),
    );
}

fn finished_tasks(text: &mut String, tasks: &[ReportTask]) {
    table(
        text,
        &["Task", "Title", "Who", "Finished", "Due date"],
        &tasks
            .iter()
            .map(|task| {
                vec![
                    task.reference.clone(),
                    task.title.clone(),
                    task.holder.clone(),
                    task.finished_on.map(date_in_words).unwrap_or_default(),
                    due_in_words(task.due, task.due_on),
                ]
            })
            .collect::<Vec<_>>(),
    );
}

fn open_tasks(text: &mut String, tasks: &[ReportTask]) {
    table(
        text,
        &["Task", "Title", "Status", "Who", "Due"],
        &tasks
            .iter()
            .map(|task| {
                vec![
                    task.reference.clone(),
                    task.title.clone(),
                    task.status.clone().unwrap_or_default(),
                    task.holder.clone(),
                    due_in_words(task.due, task.due_on),
                ]
            })
            .collect::<Vec<_>>(),
    );
}

fn sprints(text: &mut String, sprints: &[SprintOutcome]) {
    table(
        text,
        &["Sprint", "Dates", "Goal", "Outcome"],
        &sprints
            .iter()
            .map(|sprint| {
                vec![
                    sprint.name.clone(),
                    sprint_dates(sprint),
                    sprint.goal.clone(),
                    sprint_outcome(sprint),
                ]
            })
            .collect::<Vec<_>>(),
    );
}

/// "10 November 2026 to 15 November 2026", or the one date it has.
#[must_use]
pub fn sprint_dates(sprint: &SprintOutcome) -> String {
    match (sprint.start_on, sprint.end_on) {
        (Some(start), Some(end)) => {
            format!("{} to {}", date_in_words(start), date_in_words(end))
        }
        (Some(start), None) => format!("From {}", date_in_words(start)),
        (None, Some(end)) => format!("Until {}", date_in_words(end)),
        (None, None) => String::new(),
    }
}

/// "Completed: 5 of 10 points finished".
#[must_use]
pub fn sprint_outcome(sprint: &SprintOutcome) -> String {
    let unit = match (sprint.unit, sprint.committed) {
        (SprintUnit::Points, 1) => "point",
        (SprintUnit::Points, _) => "points",
        (SprintUnit::Tasks, 1) => "task",
        (SprintUnit::Tasks, _) => "tasks",
    };
    let state = match sprint.state {
        SprintState::Planned => "Planned",
        SprintState::Active => "Running",
        SprintState::Completed => "Completed",
    };
    format!(
        "{state}: {} of {} {unit} finished",
        sprint.completed, sprint.committed
    )
}

fn usage(text: &mut String, name: &str, lines: &[UsageLine], total: Option<&UsageLine>) {
    let mut rows: Vec<Vec<String>> = lines
        .iter()
        .map(|usage| {
            vec![
                usage.name.clone(),
                number_in_words(usage.tokens),
                usage.turns.to_string(),
            ]
        })
        .collect();
    if let Some(total) = total {
        rows.push(vec![
            format!("**{}**", total.name),
            format!("**{}**", number_in_words(total.tokens)),
            format!("**{}**", total.turns),
        ]);
    }
    table(text, &[name, "Tokens:", "Turns:"], &rows);
}

fn decision(text: &mut String, decision: &ReportDecision) {
    subsection(text, &one_line(&decision.question));
    let about = decision
        .task
        .as_ref()
        .map(|task| format!(" about {task}"))
        .unwrap_or_default();
    line(
        text,
        &format!(
            "Asked by {}{about}. Answered on {}.",
            decision.asked_by,
            date_in_words(decision.answered_on)
        ),
    );
    blank(text);
    if let Some(chosen) = &decision.chosen {
        line(text, &format!("**Chosen:** {}", one_line(chosen)));
        if !decision.note.is_empty() {
            blank(text);
        }
    }
    if !decision.note.is_empty() {
        line(text, &decision.note);
    }
}

fn handoff(text: &mut String, handoff: &ReportHandoff) {
    subsection(
        text,
        &format!("{} {}", handoff.reference, one_line(&handoff.title)),
    );
    line(
        text,
        &format!(
            "Handed over by {} on {}.",
            handoff.author,
            date_in_words(handoff.published_on)
        ),
    );
    blank(text);
    line(text, &handoff.summary);
    if !handoff.contract.is_empty() {
        blank(text);
        line(text, "**The contract**");
        blank(text);
        // A fence longer than any run of backticks in the contract, so that
        // the contract cannot close it.
        let fence = "`".repeat(longest_run_of_backticks(&handoff.contract).max(2) + 1);
        line(text, &fence);
        line(text, &handoff.contract);
        line(text, &fence);
    }
    if !handoff.files.is_empty() {
        blank(text);
        line(text, "**The files**");
        blank(text);
        for file in &handoff.files {
            let _ = writeln!(text, "- `{}`", file.replace('`', "'"));
        }
    }
}

fn longest_run_of_backticks(words: &str) -> usize {
    words
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_report::{PlanAgainstActual, ReportBrief, ReportUsage};
    use teamwright_domain::{ProjectId, TaskId};

    fn day(day_of_month: u32) -> CalendarDate {
        CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
    }

    /// A report with nothing in it but a project without tasks.
    fn empty_report() -> ProjectReport {
        ProjectReport {
            project_id: ProjectId::generate(),
            name: "Smart home hub".to_owned(),
            key: "SH".to_owned(),
            description: String::new(),
            made_on: day(21),
            begun_on: day(2),
            is_complete: false,
            title: "Report of the project so far".to_owned(),
            summary: vec![
                "The project was begun on 2 November 2026. It has no tasks yet.".to_owned(),
            ],
            brief: None,
            by_area: Vec::new(),
            by_holder: Vec::new(),
            finished: Vec::new(),
            plan: PlanAgainstActual {
                sentences: vec!["No task is finished yet, so nothing can be compared.".to_owned()],
                ..PlanAgainstActual::default()
            },
            sprints: Vec::new(),
            usage: ReportUsage::default(),
            decisions: Vec::new(),
            handoffs: Vec::new(),
            open: Vec::new(),
            waiting: 0,
        }
    }

    fn task(reference: &str, title: &str, due: DueOutcome) -> ReportTask {
        ReportTask {
            task_id: TaskId::generate(),
            reference: reference.to_owned(),
            title: title.to_owned(),
            area: None,
            holder: "Ada".to_owned(),
            status: Some("Done".to_owned()),
            due_on: Some(day(10)),
            finished_on: Some(day(12)),
            estimate_minutes: None,
            worked_minutes: None,
            due,
        }
    }

    #[test]
    fn a_report_without_tasks_leaves_out_every_part_that_has_nothing_to_say() {
        let text = report_in_markdown(&empty_report());

        assert_eq!(
            text,
            "# Smart home hub\n\
             \n\
             Report of the project so far, made on 21 November 2026. The key of its tasks is SH.\n\
             \n\
             ## Summary\n\
             \n\
             The project was begun on 2 November 2026. It has no tasks yet.\n\
             \n\
             ## What was done\n\
             \n\
             Nothing is finished yet.\n\
             \n\
             ## Planned against what happened\n\
             \n\
             - No task is finished yet, so nothing can be compared.\n\
             \n\
             ---\n\
             \n\
             Made with Teamwright on 21 November 2026, from what the project has stored.\n"
        );
        for heading in [
            "Sprints",
            "Decisions",
            "Results handed over",
            "Still open",
            "What the team used",
            "What was to be built",
        ] {
            assert!(!text.contains(heading), "{heading} should be left out");
        }
    }

    #[test]
    fn the_finished_tasks_are_a_table_with_how_each_met_its_due_date() {
        let mut report = empty_report();
        report.finished = vec![
            task("SH-1", "Draw the schematic", DueOutcome::Met),
            task("SH-2", "Route | the board", DueOutcome::Missed { days: 2 }),
        ];
        report.by_holder = vec![WorkShare {
            name: "Ada".to_owned(),
            finished: 2,
            open: 0,
        }];

        let text = report_in_markdown(&report);

        assert!(text.contains(
            "### By who held the work\n\n| Who | Finished | Open |\n| --- | ---: | ---: |\n| Ada | 2 | 0 |\n"
        ));
        assert!(text.contains("| SH-1 | Draw the schematic | Ada | 12 November 2026 | On time |"));
        // A bar in a title would end its cell.
        assert!(
            text.contains("| SH-2 | Route \\| the board | Ada | 12 November 2026 | 2 days late |")
        );
        assert!(!text.contains("### By area"));
    }

    #[test]
    fn the_brief_says_when_the_plan_was_approved() {
        let mut report = empty_report();
        report.brief = Some(ReportBrief {
            summary: "A hub that ties the lights together.".to_owned(),
            approved_on: day(3),
        });

        let text = report_in_markdown(&report);

        assert!(text.contains(
            "## What was to be built\n\nA hub that ties the lights together.\n\n*The plan of the Advisor, as the manager approved it on 3 November 2026.*\n"
        ));
    }

    #[test]
    fn a_decision_names_who_asked_what_was_chosen_and_what_the_manager_added() {
        let mut report = empty_report();
        report.decisions = vec![ReportDecision {
            asked_by: "Hardware engineer".to_owned(),
            task: Some("SH-4".to_owned()),
            question: "Which battery?".to_owned(),
            chosen: Some("One 18650 cell".to_owned()),
            note: "It must last a year.".to_owned(),
            answered_on: day(5),
        }];

        let text = report_in_markdown(&report);

        assert!(text.contains(
            "## Decisions of the manager\n\n### Which battery?\n\nAsked by Hardware engineer about SH-4. Answered on 5 November 2026.\n\n**Chosen:** One 18650 cell\n\nIt must last a year.\n"
        ));
    }

    #[test]
    fn a_handoff_fences_its_contract_so_that_the_contract_cannot_close_the_fence() {
        let mut report = empty_report();
        report.handoffs = vec![ReportHandoff {
            reference: "SH-3".to_owned(),
            title: "Choose the microcontroller".to_owned(),
            author: "Hardware engineer".to_owned(),
            published_on: day(6),
            summary: "The ESP32-C3 it is.".to_owned(),
            contract: "pins:\n```\nGPIO4 = SDA\n```".to_owned(),
            files: vec!["docs/mcu.md".to_owned()],
        }];

        let text = report_in_markdown(&report);

        assert!(text.contains("### SH-3 Choose the microcontroller\n\nHanded over by Hardware engineer on 6 November 2026.\n\nThe ESP32-C3 it is.\n"));
        assert!(text.contains("\n````\npins:\n```\nGPIO4 = SDA\n```\n````\n"));
        assert!(text.contains("**The files**\n\n- `docs/mcu.md`\n"));
    }

    #[test]
    fn what_is_still_open_lists_the_tasks_and_the_questions_that_wait() {
        let mut report = empty_report();
        let mut open = task("SH-5", "Print the case", DueOutcome::Late { days: 1 });
        open.status = Some("In progress".to_owned());
        open.finished_on = None;
        report.open = vec![open];
        report.waiting = 2;

        let text = report_in_markdown(&report);

        assert!(text.contains(
            "## Still open\n\n| Task | Title | Status | Who | Due |\n| --- | --- | --- | --- | --- |\n| SH-5 | Print the case | In progress | Ada | 1 day late |\n\n2 questions wait for the manager's answer.\n"
        ));
    }

    #[test]
    fn the_usage_is_by_who_with_a_total_and_by_model() {
        let mut report = empty_report();
        report.usage = ReportUsage {
            by_who: vec![UsageLine {
                name: "Ada".to_owned(),
                tokens: 1_234_567,
                turns: 3,
            }],
            by_model: vec![UsageLine {
                name: "opus".to_owned(),
                tokens: 1_234_567,
                turns: 3,
            }],
            total: UsageLine {
                name: "Everyone".to_owned(),
                tokens: 1_234_567,
                turns: 3,
            },
        };

        let text = report_in_markdown(&report);

        assert!(text.contains("| Ada | 1,234,567 | 3 |\n| **Everyone** | **1,234,567** | **3** |\n\n| Model | Tokens | Turns |"));
        assert!(text.contains("| opus | 1,234,567 | 3 |"));
    }

    #[test]
    fn the_file_is_named_after_the_project_and_the_day_without_forbidden_characters() {
        assert_eq!(
            report_file_name("Smart sensor node", day(21)),
            "Smart sensor node report 2026-11-21.md"
        );
        assert_eq!(
            report_file_name("A/B: test?  run", day(1)),
            "A B test run report 2026-11-01.md"
        );
        assert_eq!(
            report_file_name("..", day(1)),
            "Project report 2026-11-01.md"
        );
    }

    #[test]
    fn a_sprint_says_its_dates_and_what_it_finished() {
        let sprint = SprintOutcome {
            name: "Sprint 1".to_owned(),
            goal: "Choose the heart".to_owned(),
            state: SprintState::Completed,
            start_on: Some(day(2)),
            end_on: Some(day(6)),
            unit: SprintUnit::Points,
            committed: 10,
            completed: 5,
        };

        assert_eq!(sprint_dates(&sprint), "2 November 2026 to 6 November 2026");
        assert_eq!(
            sprint_outcome(&sprint),
            "Completed: 5 of 10 points finished"
        );
    }
}
