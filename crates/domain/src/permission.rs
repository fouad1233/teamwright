//! Permission requests: what a specialist asks to do beyond reading and
//! writing in the folder of its project, and what the manager answers.
//!
//! A specialist may read and write in the folder of its project and nothing
//! else (decision 0011). When it needs more, it asks, and the manager allows
//! or refuses (decision 0038). What can be allowed is a precise rule: one
//! command, a prefix of commands, or one web domain. Never "anything".
//!
//! Some things cannot be allowed at all, whatever the manager answers. The
//! rules of [`PermissionRule::new`] say which, and why. They are checked
//! here, so that no other part of the program can make such a rule.

use crate::error::DomainError;
use crate::ids::{ComputerId, MemberId, PermissionRequestId, ProjectId, TaskId, TurnId};
use crate::macros::text_enum;
use crate::teamwork::SaidIn;
use crate::time::Timestamp;

/// What a permission is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionKind {
    /// Running exactly one command line.
    Command,
    /// Running any command line that starts with these words.
    CommandPrefix,
    /// Fetching pages from one web domain.
    WebDomain,
}

text_enum!(PermissionKind, kind = "kind of permission", {
    Command => "command",
    CommandPrefix => "command_prefix",
    WebDomain => "web_domain",
});

/// Programs that are never allowed, not even as one exact command: the
/// agent program itself, whose settings a specialist must not change;
/// Teamwright; and programs that run something with other rights.
const NEVER_PROGRAMS: &[&str] = &[
    "claude",
    "teamwright",
    "sudo",
    "su",
    "doas",
    "pkexec",
    "runas",
    "gsudo",
];

/// Programs that change files. A specialist changes files with the file
/// tools, which stay inside the folder of its project; with a shell, the
/// agent program would approve some of these on its own in that folder
/// and in the folders it may only read. So they are never allowed, and a
/// turn that may run commands is denied them by name as well.
pub const FILE_CHANGING_PROGRAMS: &[&str] = &[
    "rm", "rmdir", "mv", "cp", "mkdir", "touch", "ln", "chmod", "chown", "chgrp", "dd", "tee",
    "truncate", "shred", "install",
];

/// Programs that run whatever follows them. A prefix that starts with one of
/// them would allow any command at all, so it is never a prefix.
const RUNS_ANYTHING: &[&str] = &[
    "sh",
    "bash",
    "zsh",
    "fish",
    "dash",
    "ksh",
    "csh",
    "tcsh",
    "pwsh",
    "powershell",
    "cmd",
    "python",
    "python3",
    "py",
    "node",
    "deno",
    "bun",
    "ruby",
    "perl",
    "php",
    "lua",
    "env",
    "xargs",
    "eval",
    "exec",
    "nohup",
    "timeout",
    "time",
    "nice",
    "watch",
    "find",
    "npx",
    "bunx",
    "pnpx",
    "uvx",
    "ssh",
    "scp",
    "curl",
    "wget",
    "start",
    "open",
    "xdg-open",
];

/// Characters a command may not contain. They join commands, redirect
/// output, put in the value of a variable or the output of another command,
/// or would be read as part of the rule itself (`*`, the parentheses).
const FORBIDDEN_IN_COMMANDS: &[char] = &[
    ';', '&', '|', '<', '>', '`', '$', '*', '?', '(', ')', '{', '}', '[', ']', '\\', '~', '!', '#',
    '%', '^',
];

/// A precise thing a specialist may be allowed to do: run one command, run
/// the commands that start with a prefix, or fetch from one web domain.
///
/// A rule is checked when it is made, and it cannot be made in a form that
/// reaches beyond what it shows:
///
/// - A command is one line with no way to join a second command to it, to
///   redirect what it writes, or to use a variable. It names no absolute
///   path, no path into a parent folder, no home folder and nothing of the
///   agent program's settings (`.claude`) or of Teamwright, so it cannot
///   touch Teamwright's own data or the folder it syncs through. It does
///   not run the agent program, Teamwright, or anything with other rights
///   (`sudo` and the like).
/// - A prefix is a command of at least two words, the program and what it
///   is asked to do (`npm run`, `cargo test`), and its program is not one
///   that runs whatever follows it (a shell, an interpreter, `env`,
///   `xargs`, `curl`, `rm`, and the like).
/// - A web domain is a host name with at least two parts, such as
///   `docs.rs`, or all the hosts under one (`*.example.com`). It is not an
///   address of the computer itself or of a local network.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionRule {
    kind: PermissionKind,
    text: String,
}

impl PermissionRule {
    /// The most characters of a rule.
    pub const MAX_LENGTH: usize = 300;

    /// Makes a rule of this kind from its text. Whitespace around the text
    /// is removed; a web domain is written in small letters.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::PermissionNotPossible`], with the reason in
    /// words, if the rule would allow more than it shows or something that
    /// is never allowed; or an error if it is empty or too long.
    pub fn new(kind: PermissionKind, text: &str) -> Result<Self, DomainError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(DomainError::EmptyText {
                field: "permission",
            });
        }
        let length = text.chars().count();
        if length > Self::MAX_LENGTH {
            return Err(DomainError::TextTooLong {
                field: "permission",
                max_length: Self::MAX_LENGTH,
                actual_length: length,
            });
        }
        let text = match kind {
            PermissionKind::Command => checked_command(text, false)?,
            PermissionKind::CommandPrefix => checked_command(text, true)?,
            PermissionKind::WebDomain => checked_domain(text)?,
        };
        Ok(Self { kind, text })
    }

    /// What the rule is about.
    #[must_use]
    pub const fn kind(&self) -> PermissionKind {
        self.kind
    }

    /// The command, the prefix or the domain, as the manager sees it.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the rule lets a command run.
    #[must_use]
    pub const fn runs_commands(&self) -> bool {
        matches!(
            self.kind,
            PermissionKind::Command | PermissionKind::CommandPrefix
        )
    }

    /// The rule in the syntax of the agent program's permission rules:
    /// `Bash(npm test)`, `Bash(npm run *)` or `WebFetch(domain:docs.rs)`.
    #[must_use]
    pub fn tool_rule(&self) -> String {
        match self.kind {
            PermissionKind::Command => format!("Bash({})", self.text),
            PermissionKind::CommandPrefix => format!("Bash({} *)", self.text),
            PermissionKind::WebDomain => format!("WebFetch(domain:{})", self.text),
        }
    }

    /// The rule in words, for a specialist and for the manager: "run the
    /// command `npm test`".
    #[must_use]
    pub fn in_words(&self) -> String {
        match self.kind {
            PermissionKind::Command => format!("run the command `{}`", self.text),
            PermissionKind::CommandPrefix => {
                format!("run commands that start with `{}`", self.text)
            }
            PermissionKind::WebDomain => format!("fetch web pages from {}", self.text),
        }
    }
}

fn not_possible(reason: &str) -> DomainError {
    DomainError::PermissionNotPossible {
        reason: reason.to_owned(),
    }
}

/// Checks a command or a prefix, and returns it with single spaces between
/// its words.
fn checked_command(text: &str, prefix: bool) -> Result<String, DomainError> {
    if text.chars().any(char::is_control) {
        return Err(not_possible("a command is one line"));
    }
    if let Some(forbidden) = text.chars().find(|c| FORBIDDEN_IN_COMMANDS.contains(c)) {
        return Err(not_possible(&format!(
            "a command may not contain \"{forbidden}\": it could join, redirect or widen commands"
        )));
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let Some(first) = words.first() else {
        return Err(DomainError::EmptyText {
            field: "permission",
        });
    };
    if first.contains('=') {
        return Err(not_possible(
            "a command may not start by setting a variable",
        ));
    }
    for word in &words {
        let bare = word.trim_matches(|c| c == '"' || c == '\'');
        let lower = bare.to_lowercase();
        let drive = bare.len() >= 2
            && bare.as_bytes()[1] == b':'
            && bare.as_bytes()[0].is_ascii_alphabetic();
        if bare.starts_with('/') || drive {
            return Err(not_possible(
                "a command may name only paths inside the folder of the project, not \
                 absolute paths",
            ));
        }
        if bare.split(['/', '=']).any(|part| part == "..") {
            return Err(not_possible(
                "a command may not reach into a folder above the folder of the project",
            ));
        }
        if lower.contains(".claude") || lower.contains("teamwright") {
            return Err(not_possible(
                "a command may not touch the settings of the agent program or Teamwright's \
                 own data",
            ));
        }
    }
    let program = program_name(first);
    if NEVER_PROGRAMS.contains(&program.as_str()) {
        return Err(not_possible(&format!(
            "\"{program}\" can never be allowed: it would change the agent program, \
             Teamwright, or run with other rights"
        )));
    }
    if FILE_CHANGING_PROGRAMS.contains(&program.as_str()) {
        return Err(not_possible(&format!(
            "\"{program}\" changes files; a specialist changes files with its file tools, \
             inside the folder of the project"
        )));
    }
    if prefix {
        if words.len() < 2 {
            return Err(not_possible(
                "a prefix names the program and what it is asked to do, such as \"npm run\"",
            ));
        }
        if RUNS_ANYTHING.contains(&program.as_str()) {
            return Err(not_possible(&format!(
                "\"{program}\" runs whatever follows it, so it is allowed only as one exact \
                 command"
            )));
        }
    }
    Ok(words.join(" "))
}

/// The name of a program as it is compared: in small letters, without the
/// folders before it and without `.exe`.
fn program_name(word: &str) -> String {
    let name = word.rsplit('/').next().unwrap_or(word).to_lowercase();
    name.strip_suffix(".exe")
        .map_or(name.clone(), str::to_owned)
}

/// Checks a web domain, and returns it in small letters.
fn checked_domain(text: &str) -> Result<String, DomainError> {
    let domain = text.to_lowercase();
    let host = domain.strip_prefix("*.").unwrap_or(&domain);
    let labels: Vec<&str> = host.split('.').collect();
    let well_formed = labels.iter().all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    });
    if !well_formed {
        return Err(not_possible(
            "a web domain is a host name such as docs.rs or *.example.com, without \
             \"https://\" or a path",
        ));
    }
    if labels.len() < 2 {
        return Err(not_possible(
            "a web domain has at least two parts, such as docs.rs",
        ));
    }
    let last = labels.last().copied().unwrap_or_default();
    if last.chars().all(|c| c.is_ascii_digit()) || last == "localhost" || last == "local" {
        return Err(not_possible(
            "an address of this computer or of a local network can never be allowed",
        ));
    }
    Ok(domain)
}

/// How long an allowed permission holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionScope {
    /// For the turn the answer starts, and no other.
    Once,
    /// For every turn of this specialist about this task.
    Task,
    /// For every turn of this specialist in this project, until the manager
    /// takes it back: a standing permission.
    Always,
}

text_enum!(PermissionScope, kind = "scope of a permission", {
    Once => "once",
    Task => "task",
    Always => "always",
});

/// What the manager answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Allowed, with this rule, for this long. The rule may be one the
    /// manager edited from what was asked, but it passed the same checks.
    Allowed {
        /// How long it holds.
        scope: PermissionScope,
        /// What is allowed.
        rule: PermissionRule,
    },
    /// Refused.
    Refused,
}

/// The manager's answer to a permission request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionAnswer {
    /// Allowed or refused.
    pub decision: PermissionDecision,
    /// What the manager added in words. May be empty.
    pub note: String,
    /// When the manager answered.
    pub at: Timestamp,
    /// The computer the manager answered on. An allowed permission holds
    /// only there: a command is a command of that computer.
    pub computer: ComputerId,
    /// The turn the answer started.
    pub next_turn: TurnId,
    /// When the manager took an allowed permission back, if it did.
    pub withdrawn_at: Option<Timestamp>,
}

/// The plain data of a permission request, used to recreate one that was
/// stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequestSnapshot {
    /// The identifier.
    pub id: PermissionRequestId,
    /// The project.
    pub project_id: ProjectId,
    /// Who asked, in which conversation and turn.
    pub said_in: SaidIn,
    /// The task the conversation is about, if any.
    pub task_id: Option<TaskId>,
    /// What was asked.
    pub asked: PermissionRule,
    /// Why, in the specialist's words.
    pub reason: String,
    /// When it was asked.
    pub asked_at: Timestamp,
    /// The answer, if there is one.
    pub answer: Option<PermissionAnswer>,
}

/// A specialist's request to do something it may not do on its own.
///
/// It is asked once, answered once. An allowed answer for a task or for
/// always can be taken back; a request that was refused stays refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequest {
    id: PermissionRequestId,
    project_id: ProjectId,
    said_in: SaidIn,
    task_id: Option<TaskId>,
    asked: PermissionRule,
    reason: String,
    asked_at: Timestamp,
    answer: Option<PermissionAnswer>,
}

impl PermissionRequest {
    /// The most characters of the reason.
    pub const MAX_REASON_LENGTH: usize = 1_000;

    /// Records a request. Whitespace around the reason is removed.
    ///
    /// # Errors
    ///
    /// Returns an error if the reason is empty or too long: the manager must
    /// be told why.
    pub fn ask(
        project_id: ProjectId,
        said_in: SaidIn,
        task_id: Option<TaskId>,
        asked: PermissionRule,
        reason: &str,
        now: Timestamp,
    ) -> Result<Self, DomainError> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(DomainError::EmptyText {
                field: "reason of a permission request",
            });
        }
        let length = reason.chars().count();
        if length > Self::MAX_REASON_LENGTH {
            return Err(DomainError::TextTooLong {
                field: "reason of a permission request",
                max_length: Self::MAX_REASON_LENGTH,
                actual_length: length,
            });
        }
        Ok(Self {
            id: PermissionRequestId::generate(),
            project_id,
            said_in,
            task_id,
            asked,
            reason: reason.to_owned(),
            asked_at: now,
            answer: None,
        })
    }

    /// Recreates a request from stored data.
    #[must_use]
    pub fn restore(snapshot: PermissionRequestSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            said_in: snapshot.said_in,
            task_id: snapshot.task_id,
            asked: snapshot.asked,
            reason: snapshot.reason,
            asked_at: snapshot.asked_at,
            answer: snapshot.answer,
        }
    }

    /// The identifier.
    #[must_use]
    pub const fn id(&self) -> PermissionRequestId {
        self.id
    }

    /// The project.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// Who asked, in which conversation and turn.
    #[must_use]
    pub const fn said_in(&self) -> SaidIn {
        self.said_in
    }

    /// The specialist who asked.
    #[must_use]
    pub const fn member_id(&self) -> MemberId {
        self.said_in.member_id
    }

    /// The task the conversation is about, if any.
    #[must_use]
    pub const fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    /// What was asked.
    #[must_use]
    pub const fn asked(&self) -> &PermissionRule {
        &self.asked
    }

    /// Why, in the specialist's words.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// When it was asked.
    #[must_use]
    pub const fn asked_at(&self) -> Timestamp {
        self.asked_at
    }

    /// The answer, if there is one.
    #[must_use]
    pub const fn answer(&self) -> Option<&PermissionAnswer> {
        self.answer.as_ref()
    }

    /// Whether the manager has not answered yet.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.answer.is_none()
    }

    /// Allows the request, with the rule the manager saw: what was asked, or
    /// the manager's edit of it, which passed the same checks. The answer
    /// starts `next_turn`.
    ///
    /// # Errors
    ///
    /// Returns an error if the request has been answered already.
    pub fn allow(
        &mut self,
        scope: PermissionScope,
        rule: PermissionRule,
        note: &str,
        answered: Answered,
    ) -> Result<(), DomainError> {
        self.answer_with(PermissionDecision::Allowed { scope, rule }, note, answered)
    }

    /// Refuses the request. The answer starts `next_turn`, which tells the
    /// specialist.
    ///
    /// # Errors
    ///
    /// Returns an error if the request has been answered already.
    pub fn refuse(&mut self, note: &str, answered: Answered) -> Result<(), DomainError> {
        self.answer_with(PermissionDecision::Refused, note, answered)
    }

    fn answer_with(
        &mut self,
        decision: PermissionDecision,
        note: &str,
        answered: Answered,
    ) -> Result<(), DomainError> {
        if self.answer.is_some() {
            return Err(DomainError::AlreadyAnswered);
        }
        self.answer = Some(PermissionAnswer {
            decision,
            note: note.trim().to_owned(),
            at: answered.at,
            computer: answered.computer,
            next_turn: answered.next_turn,
            withdrawn_at: None,
        });
        Ok(())
    }

    /// The rule this request allows for a while, with how long, if it was
    /// allowed for a task or for always and has not been taken back.
    #[must_use]
    pub fn standing(&self) -> Option<(PermissionScope, &PermissionRule)> {
        let answer = self.answer.as_ref()?;
        if answer.withdrawn_at.is_some() {
            return None;
        }
        match &answer.decision {
            PermissionDecision::Allowed { scope, rule } if *scope != PermissionScope::Once => {
                Some((*scope, rule))
            }
            _ => None,
        }
    }

    /// Takes back a permission that was allowed for a task or for always.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::NothingToWithdraw`] if nothing of the request
    /// stands: it is open, refused, allowed once, or taken back already.
    pub fn withdraw(&mut self, now: Timestamp) -> Result<(), DomainError> {
        if self.standing().is_none() {
            return Err(DomainError::NothingToWithdraw);
        }
        if let Some(answer) = self.answer.as_mut() {
            answer.withdrawn_at = Some(now);
        }
        Ok(())
    }

    /// The rule this request gives a turn, if it gives it one: the turn the
    /// answer started, for a permission allowed once; any turn of the same
    /// specialist about the same task, for a task; any turn of the same
    /// specialist in the project, for always. Only on the computer the
    /// manager answered on, and never once it is taken back.
    #[must_use]
    pub fn grants(&self, turn: &TurnAbout) -> Option<&PermissionRule> {
        let answer = self.answer.as_ref()?;
        let PermissionDecision::Allowed { scope, rule } = &answer.decision else {
            return None;
        };
        if answer.withdrawn_at.is_some()
            || answer.computer != turn.computer
            || self.said_in.member_id != turn.member_id
            || self.project_id != turn.project_id
        {
            return None;
        }
        let holds = match scope {
            PermissionScope::Once => answer.next_turn == turn.turn_id,
            PermissionScope::Task => self.task_id.is_some() && self.task_id == turn.task_id,
            PermissionScope::Always => true,
        };
        holds.then_some(rule)
    }
}

/// When, where and with which turn the manager answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answered {
    /// When.
    pub at: Timestamp,
    /// On which computer.
    pub computer: ComputerId,
    /// The turn the answer starts.
    pub next_turn: TurnId,
}

/// A turn about to start, as far as its permissions go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnAbout {
    /// The project.
    pub project_id: ProjectId,
    /// The specialist.
    pub member_id: MemberId,
    /// The task the conversation is about, if any.
    pub task_id: Option<TaskId>,
    /// The turn.
    pub turn_id: TurnId,
    /// The computer it runs on.
    pub computer: ComputerId,
}

/// The rules the requests give a turn, each once, in the order they were
/// asked.
#[must_use]
pub fn granted_rules(requests: &[PermissionRequest], turn: &TurnAbout) -> Vec<PermissionRule> {
    let mut sorted: Vec<&PermissionRequest> = requests.iter().collect();
    sorted.sort_by_key(|request| (request.asked_at(), request.id()));
    let mut rules: Vec<PermissionRule> = Vec::new();
    for rule in sorted.iter().filter_map(|request| request.grants(turn)) {
        if !rules.contains(rule) {
            rules.push(rule.clone());
        }
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ConversationId;

    fn command(text: &str) -> Result<PermissionRule, DomainError> {
        PermissionRule::new(PermissionKind::Command, text)
    }

    fn prefix(text: &str) -> Result<PermissionRule, DomainError> {
        PermissionRule::new(PermissionKind::CommandPrefix, text)
    }

    fn domain(text: &str) -> Result<PermissionRule, DomainError> {
        PermissionRule::new(PermissionKind::WebDomain, text)
    }

    #[test]
    fn a_command_a_prefix_and_a_domain_become_rules_of_the_agent_program() {
        assert_eq!(
            command(" npm  test ").unwrap().tool_rule(),
            "Bash(npm test)"
        );
        assert_eq!(prefix("npm run").unwrap().tool_rule(), "Bash(npm run *)");
        assert_eq!(
            domain("Docs.RS").unwrap().tool_rule(),
            "WebFetch(domain:docs.rs)"
        );
        assert_eq!(
            domain("*.example.com").unwrap().tool_rule(),
            "WebFetch(domain:*.example.com)"
        );
        assert_eq!(
            command("git commit -m \"first draft\"").unwrap().text(),
            "git commit -m \"first draft\""
        );
        assert_eq!(
            command("echo hello").unwrap().in_words(),
            "run the command `echo hello`"
        );
    }

    #[test]
    fn a_command_cannot_be_joined_redirected_or_widened() {
        for text in [
            "npm test && rm -rf build",
            "npm test; ls",
            "npm test | tee out.txt",
            "npm test > out.txt",
            "echo $(whoami)",
            "echo `whoami`",
            "echo $HOME",
            "npm run *",
            "ls ~",
            "npm test\nrm -rf build",
            "npm test &",
            "Bash(npm test)",
        ] {
            assert!(
                matches!(
                    command(text),
                    Err(DomainError::PermissionNotPossible { .. })
                ),
                "{text} was allowed"
            );
        }
    }

    #[test]
    fn a_command_cannot_reach_outside_the_folder_or_into_teamwrights_data() {
        for text in [
            "cat /etc/passwd",
            "type C:\\Users\\sample\\secret.txt",
            "dir C:/Users",
            "cp notes.md ../other/notes.md",
            "cat .claude/settings.json",
            "ls Teamwright",
            "NODE_ENV=production npm test",
        ] {
            assert!(command(text).is_err(), "{text} was allowed");
        }
    }

    #[test]
    fn some_programs_are_never_allowed() {
        for text in [
            "claude config set",
            "claude",
            "sudo make install",
            "teamwright --team-bus",
            "/usr/bin/sudo ls",
            "su root",
            "rm -rf build",
            "mv a.txt b.txt",
            "tee notes.md",
        ] {
            assert!(command(text).is_err(), "{text} was allowed");
        }
    }

    #[test]
    fn a_prefix_names_a_subcommand_of_a_program_that_does_not_run_anything() {
        assert!(prefix("npm").is_err());
        assert!(prefix("cargo").is_err());
        for text in [
            "bash -c",
            "python -m",
            "env FOO",
            "rm -rf",
            "curl -s",
            "npx tsc",
            "xargs grep",
        ] {
            assert!(prefix(text).is_err(), "{text} was allowed as a prefix");
        }
        assert!(prefix("cargo test").is_ok());
        // As one exact command, an interpreter is shown in full and may run.
        assert!(command("python3 scripts/check.py").is_ok());
    }

    #[test]
    fn a_domain_is_a_host_name_of_the_web() {
        for text in [
            "https://docs.rs",
            "docs.rs/serde",
            "localhost",
            "127.0.0.1",
            "printer.local",
            "*",
            "*.com",
            "rs",
            "exa mple.com",
            "-bad.com",
        ] {
            assert!(domain(text).is_err(), "{text} was allowed");
        }
    }

    #[test]
    fn a_rule_is_not_empty_or_too_long() {
        assert!(command("  ").is_err());
        assert!(command(&"a".repeat(301)).is_err());
    }

    fn said_in() -> SaidIn {
        SaidIn {
            member_id: MemberId::generate(),
            conversation_id: ConversationId::generate(),
            turn_id: TurnId::generate(),
        }
    }

    fn at(minute: i64) -> Timestamp {
        Timestamp::from_timestamp(1_800_000_000 + minute * 60, 0).unwrap()
    }

    struct Setting {
        project: ProjectId,
        task: TaskId,
        computer: ComputerId,
        said_in: SaidIn,
    }

    fn setting() -> Setting {
        Setting {
            project: ProjectId::generate(),
            task: TaskId::generate(),
            computer: ComputerId::generate(),
            said_in: said_in(),
        }
    }

    fn request(setting: &Setting) -> PermissionRequest {
        PermissionRequest::ask(
            setting.project,
            setting.said_in,
            Some(setting.task),
            command("npm test").unwrap(),
            "The tests show whether the parser works.",
            at(0),
        )
        .unwrap()
    }

    fn turn(setting: &Setting, task: Option<TaskId>, turn_id: TurnId) -> TurnAbout {
        TurnAbout {
            project_id: setting.project,
            member_id: setting.said_in.member_id,
            task_id: task,
            turn_id,
            computer: setting.computer,
        }
    }

    #[test]
    fn a_request_needs_a_reason() {
        let setting = setting();
        assert!(
            PermissionRequest::ask(
                setting.project,
                setting.said_in,
                None,
                command("npm test").unwrap(),
                "  ",
                at(0)
            )
            .is_err()
        );
    }

    #[test]
    fn allowed_once_holds_for_the_turn_the_answer_started_and_no_other() {
        let setting = setting();
        let mut asked = request(&setting);
        let next = TurnId::generate();
        asked
            .allow(
                PermissionScope::Once,
                command("npm test").unwrap(),
                "",
                Answered {
                    at: at(5),
                    computer: setting.computer,
                    next_turn: next,
                },
            )
            .unwrap();

        assert!(
            asked
                .grants(&turn(&setting, Some(setting.task), next))
                .is_some()
        );
        assert!(
            asked
                .grants(&turn(&setting, Some(setting.task), TurnId::generate()))
                .is_none()
        );
        assert!(asked.standing().is_none());
        assert_eq!(asked.withdraw(at(6)), Err(DomainError::NothingToWithdraw));
    }

    #[test]
    fn allowed_for_a_task_holds_for_that_task_and_always_for_every_turn_until_taken_back() {
        let setting = setting();
        let mut for_task = request(&setting);
        let answered = Answered {
            at: at(5),
            computer: setting.computer,
            next_turn: TurnId::generate(),
        };
        for_task
            .allow(
                PermissionScope::Task,
                prefix("npm run").unwrap(),
                "",
                answered,
            )
            .unwrap();
        assert!(
            for_task
                .grants(&turn(&setting, Some(setting.task), TurnId::generate()))
                .is_some()
        );
        assert!(
            for_task
                .grants(&turn(
                    &setting,
                    Some(TaskId::generate()),
                    TurnId::generate()
                ))
                .is_none()
        );
        assert!(
            for_task
                .grants(&turn(&setting, None, TurnId::generate()))
                .is_none()
        );

        let mut always = request(&setting);
        always
            .allow(
                PermissionScope::Always,
                command("npm test").unwrap(),
                "",
                answered,
            )
            .unwrap();
        let any_turn = turn(&setting, None, TurnId::generate());
        assert!(always.grants(&any_turn).is_some());

        always.withdraw(at(9)).unwrap();
        assert!(always.grants(&any_turn).is_none());
        assert_eq!(always.withdraw(at(10)), Err(DomainError::NothingToWithdraw));
    }

    #[test]
    fn a_permission_holds_only_for_its_specialist_project_and_computer() {
        let setting = setting();
        let mut always = request(&setting);
        always
            .allow(
                PermissionScope::Always,
                command("npm test").unwrap(),
                "",
                Answered {
                    at: at(5),
                    computer: setting.computer,
                    next_turn: TurnId::generate(),
                },
            )
            .unwrap();
        let mut elsewhere = turn(&setting, None, TurnId::generate());
        elsewhere.computer = ComputerId::generate();
        assert!(always.grants(&elsewhere).is_none());
        let mut other_member = turn(&setting, None, TurnId::generate());
        other_member.member_id = MemberId::generate();
        assert!(always.grants(&other_member).is_none());
        let mut other_project = turn(&setting, None, TurnId::generate());
        other_project.project_id = ProjectId::generate();
        assert!(always.grants(&other_project).is_none());
    }

    #[test]
    fn refused_grants_nothing_and_cannot_be_answered_again() {
        let setting = setting();
        let mut refused = request(&setting);
        let answered = Answered {
            at: at(5),
            computer: setting.computer,
            next_turn: TurnId::generate(),
        };
        refused.refuse(" Not on this computer. ", answered).unwrap();
        assert_eq!(refused.answer().unwrap().note, "Not on this computer.");
        assert!(
            refused
                .grants(&turn(&setting, Some(setting.task), answered.next_turn))
                .is_none()
        );
        assert_eq!(
            refused.allow(
                PermissionScope::Always,
                command("npm test").unwrap(),
                "",
                answered
            ),
            Err(DomainError::AlreadyAnswered)
        );
        assert_eq!(refused.withdraw(at(6)), Err(DomainError::NothingToWithdraw));
    }

    #[test]
    fn the_rules_of_a_turn_are_given_once_each_in_the_order_they_were_asked() {
        let setting = setting();
        let answered = Answered {
            at: at(5),
            computer: setting.computer,
            next_turn: TurnId::generate(),
        };
        let mut first = request(&setting);
        first
            .allow(
                PermissionScope::Always,
                command("npm test").unwrap(),
                "",
                answered,
            )
            .unwrap();
        let mut second = PermissionRequest::ask(
            setting.project,
            setting.said_in,
            None,
            domain("docs.rs").unwrap(),
            "The documentation of serde.",
            at(1),
        )
        .unwrap();
        second
            .allow(
                PermissionScope::Always,
                domain("docs.rs").unwrap(),
                "",
                answered,
            )
            .unwrap();
        let mut again = request(&setting);
        again
            .allow(
                PermissionScope::Always,
                command("npm test").unwrap(),
                "",
                answered,
            )
            .unwrap();

        let rules = granted_rules(
            &[second, again, first],
            &turn(&setting, None, TurnId::generate()),
        );
        assert_eq!(
            rules,
            [command("npm test").unwrap(), domain("docs.rs").unwrap()]
        );
    }
}
