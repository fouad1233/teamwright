//! Permission requests: what a specialist asks to do beyond reading and
//! writing in the folder of its project, and what the manager answers.
//!
//! A specialist may read and write in the folder of its project and nothing
//! else (decision 0011). When it needs more, it asks, and the manager allows
//! or refuses (decision 0038). What can be allowed is a precise rule: one
//! exact command, or one web domain. Never "anything", and never a prefix
//! of commands: a prefix constrains nothing that follows it.
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
///
/// A third kind, a prefix of commands (`command_prefix`), existed for one
/// day and was taken out after a review: what follows a prefix can change
/// what the program does. A stored request of that kind is not read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionKind {
    /// Running exactly one command line.
    Command,
    /// Fetching pages from one web domain.
    WebDomain,
}

text_enum!(PermissionKind, kind = "kind of permission", {
    Command => "command",
    WebDomain => "web_domain",
});

/// Programs that are never allowed, anywhere in a command: the agent
/// program itself, whose settings a specialist must not change;
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
/// and in the folders it may only read. So a command never starts with
/// one, and a turn that may run commands is denied them by name as well.
pub const FILE_CHANGING_PROGRAMS: &[&str] = &[
    "rm", "rmdir", "mv", "cp", "mkdir", "touch", "ln", "chmod", "chown", "chgrp", "dd", "tee",
    "truncate", "shred", "install",
];

/// Programs that only start the program named after them. The agent program
/// looks past some of them when it matches a command against a rule, and
/// every check here would be about the wrong program, so a command never
/// starts with one: the manager reads the program that really runs.
const WRAPPERS: &[&str] = &[
    "env", "xargs", "eval", "exec", "nohup", "timeout", "time", "nice", "ionice", "watch",
    "stdbuf", "setsid", "command", "builtin",
];

/// Characters a command may not contain. They join commands, redirect
/// output, put in the value of a variable or the output of another command,
/// quote or escape (so that the shell would read another word than the one
/// checked here), or would be read as part of the rule itself (`*`, the
/// parentheses).
const FORBIDDEN_IN_COMMANDS: &[char] = &[
    ';', '&', '|', '<', '>', '`', '$', '*', '?', '(', ')', '{', '}', '[', ']', '\\', '~', '!', '#',
    '%', '^', '"', '\'',
];

/// A precise thing a specialist may be allowed to do: run one command, or
/// fetch from one web domain.
///
/// A rule is checked when it is made, and it cannot be made in a form that
/// reaches beyond what it shows:
///
/// - A command is one line of plain keyboard characters, words separated by
///   spaces, with no quote, no backslash and no way to join a second
///   command to it, to redirect what it writes, or to use a variable. So the
///   shell reads exactly the words checked here. It names no absolute path,
///   no drive, no home folder and no `..`, wherever in a word (also after
///   `=` or right after a one-letter option, as in `-o/etc/x`), nothing of
///   the agent program's settings (`.claude`) and nothing of Teamwright, so
///   it cannot name Teamwright's own data or the folder it syncs through.
///   It does not run the agent program, Teamwright, or anything with other
///   rights (`sudo` and the like), it does not start with a program that
///   changes files, and it does not start with a program that only starts
///   another (`env`, `nohup`, …).
/// - A web domain is a host name with at least two parts, such as
///   `docs.rs`, or all the hosts under one (`*.example.com`). It is not an
///   address of the computer itself or of a local network.
///
/// These checks narrow what a specialist can ask for; they are not a
/// boundary around a program that runs. Most programs worth allowing run
/// files of the project, which the specialist can change. That boundary is
/// the sandbox a turn with a command is started in (decision 0038).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionRule {
    kind: PermissionKind,
    text: String,
}

impl PermissionRule {
    /// The most characters of a rule.
    pub const MAX_LENGTH: usize = 300;

    /// Makes a rule of this kind from its text. Spaces around the text are
    /// removed; a web domain is written in small letters.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::PermissionNotPossible`], with the reason in
    /// words, if the rule would allow more than it shows or something that
    /// is never allowed; or an error if it is empty or too long.
    pub fn new(kind: PermissionKind, text: &str) -> Result<Self, DomainError> {
        let text = text.trim_matches(' ');
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
            PermissionKind::Command => checked_command(text)?,
            PermissionKind::WebDomain => checked_domain(text)?,
        };
        Ok(Self { kind, text })
    }

    /// What the rule is about.
    #[must_use]
    pub const fn kind(&self) -> PermissionKind {
        self.kind
    }

    /// The command or the domain, as the manager sees it.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the rule lets a command run.
    #[must_use]
    pub const fn runs_commands(&self) -> bool {
        matches!(self.kind, PermissionKind::Command)
    }

    /// The rule in the syntax of the agent program's permission rules:
    /// `Bash(npm test)` or `WebFetch(domain:docs.rs)`.
    #[must_use]
    pub fn tool_rule(&self) -> String {
        match self.kind {
            PermissionKind::Command => format!("Bash({})", self.text),
            PermissionKind::WebDomain => format!("WebFetch(domain:{})", self.text),
        }
    }

    /// The rule in words, for a specialist and for the manager: "run the
    /// command `npm test`".
    #[must_use]
    pub fn in_words(&self) -> String {
        match self.kind {
            PermissionKind::Command => format!("run the command `{}`", self.text),
            PermissionKind::WebDomain => format!("fetch web pages from {}", self.text),
        }
    }
}

fn not_possible(reason: &str) -> DomainError {
    DomainError::PermissionNotPossible {
        reason: reason.to_owned(),
    }
}

/// Checks a command, and returns it with single spaces between its words.
///
/// Every check reads the text as the shell would: nothing in it can quote,
/// escape or join, so each word below is a word the shell hands to the
/// program, letter for letter.
fn checked_command(text: &str) -> Result<String, DomainError> {
    if let Some(odd) = text.chars().find(|c| *c != ' ' && !c.is_ascii_graphic()) {
        let reason = if odd == '\n' || odd == '\r' {
            "a command is one line".to_owned()
        } else if odd == '\t' {
            "a command separates its words with spaces, not tabs".to_owned()
        } else if odd.is_control() {
            "a command may not contain control characters".to_owned()
        } else {
            format!(
                "a command is written in plain keyboard characters, and {odd} is not one: a \
                 letter that looks like another could hide what runs"
            )
        };
        return Err(not_possible(&reason));
    }
    if let Some(forbidden) = text.chars().find(|c| FORBIDDEN_IN_COMMANDS.contains(c)) {
        return Err(not_possible(&format!(
            "a command may not contain {forbidden}: it could quote, join, redirect or widen \
             commands, so that what runs is not what you read"
        )));
    }
    let words: Vec<&str> = text.split(' ').filter(|word| !word.is_empty()).collect();
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
        check_word(word)?;
    }
    let program = program_name(first);
    if WRAPPERS.contains(&program.as_str()) {
        return Err(not_possible(&format!(
            "a command may not start with {program}, which only starts another program; ask \
             for the program itself"
        )));
    }
    if FILE_CHANGING_PROGRAMS.contains(&program.as_str()) {
        return Err(not_possible(&format!(
            "{program} changes files; a specialist changes files with its file tools, inside \
             the folder of the project"
        )));
    }
    // Anywhere, also as the value of an option (`core.editor=claude`): a
    // program that runs another runs it from such a value.
    if let Some(never) = words
        .iter()
        .flat_map(|word| word.split(['=', ',', ':']))
        .map(program_name)
        .find(|name| NEVER_PROGRAMS.contains(&name.as_str()))
    {
        return Err(not_possible(&format!(
            "{never} can never be allowed: it would change the agent program, Teamwright, or \
             run with other rights"
        )));
    }
    Ok(words.join(" "))
}

/// Checks one word of a command for a path that leaves the folder of the
/// project, wherever in the word it starts.
fn check_word(word: &str) -> Result<(), DomainError> {
    let outside = || {
        not_possible(
            "a command may name only paths inside the folder of the project: no absolute path, \
             no drive and no home folder, also not after = or an option",
        )
    };
    if word.contains("..") {
        return Err(not_possible(
            "a command may not contain .., which reaches into a folder above the folder of the \
             project",
        ));
    }
    let lower = word.to_ascii_lowercase();
    if lower.contains(".claude") || lower.contains("teamwright") {
        return Err(not_possible(
            "a command may not touch the settings of the agent program or Teamwright's own data",
        ));
    }
    let bytes = word.as_bytes();
    // A slash is part of a relative path only right after a letter, a digit
    // or one of . _ - of a name. Anywhere else (at the start, after =, :,
    // @, a comma or another slash) a path starts there that may be absolute.
    let stray_slash = bytes.iter().enumerate().any(|(place, byte)| {
        *byte == b'/'
            && !(place > 0
                && matches!(
                    bytes[place - 1],
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-'
                ))
    });
    if stray_slash {
        return Err(outside());
    }
    // An option glued to an absolute path: -o/etc/x, -I/usr, -xf/x,
    // --output/x.
    if let Some((before, _)) = word.split_once('/')
        && before.starts_with('-')
        && before
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic() || byte == b'-')
    {
        return Err(outside());
    }
    // A drive, C:, at the start of the word, after = or a comma, or glued
    // to a one-letter option.
    let mut starts = vec![0];
    starts.extend(
        bytes
            .iter()
            .enumerate()
            .filter(|(_, byte)| matches!(byte, b'=' | b','))
            .map(|(place, _)| place + 1),
    );
    if bytes.len() > 2 && bytes[0] == b'-' && bytes[1].is_ascii_alphabetic() {
        starts.push(2);
    }
    let drive = starts.iter().any(|start| {
        bytes.get(*start).is_some_and(u8::is_ascii_alphabetic)
            && bytes.get(start + 1) == Some(&b':')
    });
    if drive || word.contains(":/") {
        return Err(outside());
    }
    Ok(())
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

/// Why a specialist can be given no command at all, whatever the manager
/// answered: a command runs in a sandbox or not at all, never next to a
/// secret, and never in a turn nobody asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoCommands {
    /// The computer has no sandbox for commands. Claude Code has none on
    /// Windows, and a command outside one can do anything the manager can.
    NoSandbox,
    /// The specialist has a connector that is handed a secret in an
    /// environment variable. A command could read it.
    SecretInReach,
    /// The turn was started by the coordinator to answer the messages of
    /// teammates: nobody asked for it, and nobody watches it.
    ForTeammates,
}

impl NoCommands {
    /// Why, in words, for the manager and the specialist.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::NoSandbox => {
                "a command is allowed only where it runs in a sandbox, and Claude Code has no \
                 sandbox on Windows"
            }
            Self::SecretInReach => {
                "this specialist has a connector that is given a secret in an environment \
                 variable, and a command could read it; take that connector from the \
                 specialist first"
            }
            Self::ForTeammates => {
                "a turn that answers teammates is started by the coordinator, not by the \
                 manager, and runs no command"
            }
        }
    }

    /// The reason as the error of an answer that tried to allow a command.
    #[must_use]
    pub fn as_error(self) -> DomainError {
        not_possible(self.reason())
    }
}

/// How long an allowed permission holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionScope {
    /// For the turn the answer starts, and no other.
    Once,
    /// For every turn of this specialist about this task.
    Task,
    /// For every turn of this specialist in this project, until the manager
    /// takes it back: a standing permission. Only for a web domain; a
    /// command is never allowed always.
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
    /// The computer the manager answered on, as the answer says. An allowed
    /// permission holds only there: a command is a command of that computer.
    pub computer: ComputerId,
    /// The turn the answer started.
    pub next_turn: TurnId,
    /// When the manager took an allowed permission back, if it did.
    pub withdrawn_at: Option<Timestamp>,
    /// Whether the answer was written on the computer that reads it, as its
    /// own log of operations shows, and not taken in from the sync folder.
    /// Anybody who can write in the sync folder can write an answer that
    /// names any computer, so `computer` alone proves nothing. Storage sets
    /// it when it reads; it is never stored or synced. An answer made in
    /// memory is made here.
    pub answered_here: bool,
}

impl PermissionAnswer {
    /// Whether the answer was given on `computer`, and is known to be: it
    /// says so, and the log of `computer` shows that it wrote it.
    #[must_use]
    pub fn given_on(&self, computer: ComputerId) -> bool {
        self.answered_here && self.computer == computer
    }
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
    /// Returns an error if the request has been answered already, or
    /// [`DomainError::PermissionNotPossible`] if a command is to be allowed
    /// always: a command is allowed once or for a task, never for every
    /// turn to come.
    pub fn allow(
        &mut self,
        scope: PermissionScope,
        rule: PermissionRule,
        note: &str,
        answered: Answered,
    ) -> Result<(), DomainError> {
        if self.answer.is_some() {
            return Err(DomainError::AlreadyAnswered);
        }
        if rule.runs_commands() && scope == PermissionScope::Always {
            return Err(not_possible(
                "a command can be allowed once or for this task, never always",
            ));
        }
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
            answered_here: true,
        });
        Ok(())
    }

    /// The rule this request allows for a while, with how long, if it was
    /// allowed for a task or for always and has not been taken back. A
    /// command allowed always is among them, so that it can be taken back,
    /// although it grants nothing ([`Self::is_no_longer_valid`]).
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

    /// Whether the request stands but grants nothing any more: a command
    /// allowed always, before commands could no longer be. It can only be
    /// taken back.
    #[must_use]
    pub fn is_no_longer_valid(&self) -> bool {
        matches!(
            self.standing(),
            Some((PermissionScope::Always, rule)) if rule.runs_commands()
        )
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
    /// specialist in the project, for a web domain allowed always. Only on
    /// the computer that wrote the answer itself, and never once it is
    /// taken back.
    ///
    /// A command is given to no turn that can run none
    /// ([`TurnAbout::no_commands`]), and a command allowed always, which
    /// could be stored before that was refused, to none at all.
    #[must_use]
    pub fn grants(&self, turn: &TurnAbout) -> Option<&PermissionRule> {
        let answer = self.answer.as_ref()?;
        let PermissionDecision::Allowed { scope, rule } = &answer.decision else {
            return None;
        };
        if answer.withdrawn_at.is_some()
            || !answer.given_on(turn.computer)
            || self.said_in.member_id != turn.member_id
            || self.project_id != turn.project_id
        {
            return None;
        }
        if rule.runs_commands() && (turn.no_commands.is_some() || *scope == PermissionScope::Always)
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
    /// Why the turn can run no command, whatever was allowed, if it can
    /// run none.
    pub no_commands: Option<NoCommands>,
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

    fn domain(text: &str) -> Result<PermissionRule, DomainError> {
        PermissionRule::new(PermissionKind::WebDomain, text)
    }

    fn refused(texts: &[&str]) {
        for text in texts {
            assert!(
                matches!(
                    command(text),
                    Err(DomainError::PermissionNotPossible { .. })
                ),
                "{text:?} was allowed"
            );
        }
    }

    #[test]
    fn a_command_and_a_domain_become_rules_of_the_agent_program() {
        assert_eq!(
            command(" npm  test ").unwrap().tool_rule(),
            "Bash(npm test)"
        );
        assert_eq!(
            domain("Docs.RS").unwrap().tool_rule(),
            "WebFetch(domain:docs.rs)"
        );
        assert_eq!(
            domain("*.example.com").unwrap().tool_rule(),
            "WebFetch(domain:*.example.com)"
        );
        assert_eq!(
            command("echo hello").unwrap().in_words(),
            "run the command `echo hello`"
        );
        for text in [
            "npm test",
            "cargo test --workspace",
            "npm run test:unit",
            "python3 tools/check.py",
            "pytest tests/test_parser.py -q",
            "make -C firmware",
            "cargo build --features=fast,small",
            "git log -1 --oneline",
            "./gradlew build",
            "npx tsc --noEmit",
        ] {
            assert!(command(text).is_ok(), "{text} was refused");
        }
    }

    #[test]
    fn there_is_no_prefix_of_commands() {
        assert!("command_prefix".parse::<PermissionKind>().is_err());
        assert_eq!(command("npm run").unwrap().tool_rule(), "Bash(npm run)");
    }

    #[test]
    fn a_command_cannot_be_joined_redirected_or_widened() {
        refused(&[
            "npm test && rm -rf build",
            "npm test; ls",
            "npm test | tee out.txt",
            "npm test > out.txt",
            "echo $(whoami)",
            "echo `whoami`",
            "echo $HOME",
            "npm run *",
            "ls ~",
            "npm test &",
            "Bash(npm test)",
        ]);
    }

    #[test]
    fn a_quote_or_a_backslash_anywhere_is_refused() {
        // The shell removes quotes and backslashes; the checks would read
        // another word than the one that runs.
        refused(&[
            "\"claude\" --dangerously-skip-permissions -p x",
            "'sudo' ls",
            "\"rm\" -r src",
            "\"bash\" -c x",
            "py\"\"thon3 -c x",
            "cat .cla\"\"ude/settings.json",
            "git commit -m \"first draft\"",
            "git commit -m 'first draft'",
            "c\\laude -p x",
            "echo `id`",
        ]);
    }

    #[test]
    fn only_plain_characters_on_one_line_are_allowed() {
        refused(&[
            "npm test\nrm -rf build",
            "npm test\r\nls",
            "npm\ttest",
            "npm test\u{0}",
            "npm test\u{1b}[2J",
            "npm t\u{0435}st",
            "c\u{0142}aude -p x",
            "npm\u{a0}test",
        ]);
    }

    #[test]
    fn an_absolute_path_a_drive_a_home_or_dots_are_refused_anywhere_in_a_word() {
        refused(&[
            "cat /etc/passwd",
            "git log -1 --output=/home/u/x",
            "git log --output=/home/u/.bashrc",
            "gcc -o/home/u/x main.c",
            "cc -I/usr/include main.c",
            "tar -xf/tmp/x.tar",
            "tool --output/home/u/x",
            "tool --out=C:/x",
            "dir C:/Users",
            "tool --out=C:x",
            "tool -oC:x",
            "tool a,/etc/x",
            "tool --config=x:/etc/y",
            "curl https://example.com",
            "tool @/etc/passwd",
            "ls dir//etc",
            "cp notes.md ../other/notes.md",
            "git log main..HEAD",
            "ls ~/x",
            "tool --home=~",
        ]);
    }

    #[test]
    fn nothing_of_claude_code_or_teamwright_is_named() {
        refused(&[
            "cat .claude/settings.json",
            "ls Teamwright",
            "cat notes/.CLAUDE",
            "npm exec claude",
            "npx claude -p x",
            "git -c core.editor=claude commit",
        ]);
    }

    #[test]
    fn some_programs_are_never_allowed_and_wrappers_never_start_a_command() {
        refused(&[
            "claude config set",
            "claude",
            "sudo make install",
            "teamwright --team-bus",
            "bin/sudo ls",
            "su root",
            "npm test sudo",
            "rm -rf build",
            "mv a.txt b.txt",
            "tee notes.md",
            "env FOO=bar npm test",
            "nohup npm test",
            "timeout 5 npm test",
            "xargs grep x",
            "NODE_ENV=production npm test",
        ]);
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

    #[test]
    fn why_no_command_is_said_in_words() {
        for no in [
            NoCommands::NoSandbox,
            NoCommands::SecretInReach,
            NoCommands::ForTeammates,
        ] {
            assert!(matches!(
                no.as_error(),
                DomainError::PermissionNotPossible { reason } if reason == no.reason()
            ));
        }
        assert!(NoCommands::SecretInReach.reason().contains("secret"));
        assert!(NoCommands::NoSandbox.reason().contains("Windows"));
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

    fn request_for(setting: &Setting, rule: PermissionRule) -> PermissionRequest {
        PermissionRequest::ask(
            setting.project,
            setting.said_in,
            Some(setting.task),
            rule,
            "The tests show whether the parser works.",
            at(0),
        )
        .unwrap()
    }

    fn request(setting: &Setting) -> PermissionRequest {
        request_for(setting, command("npm test").unwrap())
    }

    fn turn(setting: &Setting, task: Option<TaskId>, turn_id: TurnId) -> TurnAbout {
        TurnAbout {
            project_id: setting.project,
            member_id: setting.said_in.member_id,
            task_id: task,
            turn_id,
            computer: setting.computer,
            no_commands: None,
        }
    }

    fn answered(setting: &Setting) -> Answered {
        Answered {
            at: at(5),
            computer: setting.computer,
            next_turn: TurnId::generate(),
        }
    }

    /// A stored answer as it was before commands could no longer be allowed
    /// always: made by restoring, as storage does.
    fn stored_always(setting: &Setting, rule: PermissionRule) -> PermissionRequest {
        let asked = request_for(setting, rule.clone());
        PermissionRequest::restore(PermissionRequestSnapshot {
            id: asked.id(),
            project_id: asked.project_id(),
            said_in: asked.said_in(),
            task_id: asked.task_id(),
            asked: rule.clone(),
            reason: asked.reason().to_owned(),
            asked_at: asked.asked_at(),
            answer: Some(PermissionAnswer {
                decision: PermissionDecision::Allowed {
                    scope: PermissionScope::Always,
                    rule,
                },
                note: String::new(),
                at: at(5),
                computer: setting.computer,
                next_turn: TurnId::generate(),
                withdrawn_at: None,
                answered_here: true,
            }),
        })
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
        let answered = answered(&setting);
        asked
            .allow(
                PermissionScope::Once,
                command("npm test").unwrap(),
                "",
                answered,
            )
            .unwrap();

        assert!(
            asked
                .grants(&turn(&setting, Some(setting.task), answered.next_turn))
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
    fn allowed_for_a_task_holds_only_for_turns_about_that_task() {
        let setting = setting();
        let mut for_task = request(&setting);
        for_task
            .allow(
                PermissionScope::Task,
                command("npm test").unwrap(),
                "",
                answered(&setting),
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
        for_task.withdraw(at(9)).unwrap();
        assert!(
            for_task
                .grants(&turn(&setting, Some(setting.task), TurnId::generate()))
                .is_none()
        );
    }

    #[test]
    fn a_command_is_never_allowed_always_and_a_domain_may_be() {
        let setting = setting();
        let mut asked = request(&setting);
        assert!(matches!(
            asked.allow(
                PermissionScope::Always,
                command("npm test").unwrap(),
                "",
                answered(&setting)
            ),
            Err(DomainError::PermissionNotPossible { .. })
        ));
        assert!(asked.is_open());

        let mut docs = request_for(&setting, domain("docs.rs").unwrap());
        docs.allow(
            PermissionScope::Always,
            domain("docs.rs").unwrap(),
            "",
            answered(&setting),
        )
        .unwrap();
        let any_turn = turn(&setting, None, TurnId::generate());
        assert!(docs.grants(&any_turn).is_some());
        assert!(!docs.is_no_longer_valid());
        docs.withdraw(at(9)).unwrap();
        assert!(docs.grants(&any_turn).is_none());
        assert_eq!(docs.withdraw(at(10)), Err(DomainError::NothingToWithdraw));
    }

    #[test]
    fn a_stored_command_allowed_always_grants_nothing_and_can_be_taken_back() {
        let setting = setting();
        let mut always = stored_always(&setting, command("npm test").unwrap());
        assert!(
            always
                .grants(&turn(&setting, Some(setting.task), TurnId::generate()))
                .is_none()
        );
        assert!(always.is_no_longer_valid());
        assert!(always.standing().is_some());
        always.withdraw(at(9)).unwrap();
        assert!(!always.is_no_longer_valid());
    }

    #[test]
    fn a_turn_that_can_run_no_command_is_given_no_command_but_a_domain() {
        let setting = setting();
        let mut for_task = request(&setting);
        for_task
            .allow(
                PermissionScope::Task,
                command("npm test").unwrap(),
                "",
                answered(&setting),
            )
            .unwrap();
        let mut docs = request_for(&setting, domain("docs.rs").unwrap());
        docs.allow(
            PermissionScope::Task,
            domain("docs.rs").unwrap(),
            "",
            answered(&setting),
        )
        .unwrap();
        for no in [
            NoCommands::ForTeammates,
            NoCommands::SecretInReach,
            NoCommands::NoSandbox,
        ] {
            let mut about = turn(&setting, Some(setting.task), TurnId::generate());
            about.no_commands = Some(no);
            assert!(for_task.grants(&about).is_none(), "{no:?}");
            assert!(docs.grants(&about).is_some(), "{no:?}");
        }
    }

    #[test]
    fn a_permission_holds_only_for_its_specialist_project_and_computer() {
        let setting = setting();
        let mut docs = request_for(&setting, domain("docs.rs").unwrap());
        docs.allow(
            PermissionScope::Always,
            domain("docs.rs").unwrap(),
            "",
            answered(&setting),
        )
        .unwrap();
        let mut elsewhere = turn(&setting, None, TurnId::generate());
        elsewhere.computer = ComputerId::generate();
        assert!(docs.grants(&elsewhere).is_none());
        let mut other_member = turn(&setting, None, TurnId::generate());
        other_member.member_id = MemberId::generate();
        assert!(docs.grants(&other_member).is_none());
        let mut other_project = turn(&setting, None, TurnId::generate());
        other_project.project_id = ProjectId::generate();
        assert!(docs.grants(&other_project).is_none());
    }

    #[test]
    fn an_answer_this_computer_did_not_write_grants_nothing_here() {
        let setting = setting();
        let mut for_task = request(&setting);
        for_task
            .allow(
                PermissionScope::Task,
                command("npm test").unwrap(),
                "",
                answered(&setting),
            )
            .unwrap();
        let about = turn(&setting, Some(setting.task), TurnId::generate());
        assert!(for_task.grants(&about).is_some());
        // The same answer, naming this computer, but taken in from the sync
        // folder.
        let mut snapshot_answer = for_task.answer().unwrap().clone();
        snapshot_answer.answered_here = false;
        let synced = PermissionRequest::restore(PermissionRequestSnapshot {
            id: for_task.id(),
            project_id: for_task.project_id(),
            said_in: for_task.said_in(),
            task_id: for_task.task_id(),
            asked: for_task.asked().clone(),
            reason: for_task.reason().to_owned(),
            asked_at: for_task.asked_at(),
            answer: Some(snapshot_answer),
        });
        assert!(synced.grants(&about).is_none());
        assert!(!synced.answer().unwrap().given_on(setting.computer));
    }

    #[test]
    fn refused_grants_nothing_and_cannot_be_answered_again() {
        let setting = setting();
        let mut refused = request(&setting);
        let answered = answered(&setting);
        refused.refuse(" Not on this computer. ", answered).unwrap();
        assert_eq!(refused.answer().unwrap().note, "Not on this computer.");
        assert!(
            refused
                .grants(&turn(&setting, Some(setting.task), answered.next_turn))
                .is_none()
        );
        assert_eq!(
            refused.allow(
                PermissionScope::Task,
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
        let mut first = request(&setting);
        first
            .allow(
                PermissionScope::Task,
                command("npm test").unwrap(),
                "",
                answered(&setting),
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
                answered(&setting),
            )
            .unwrap();
        let mut again = request(&setting);
        again
            .allow(
                PermissionScope::Task,
                command("npm test").unwrap(),
                "",
                answered(&setting),
            )
            .unwrap();

        let rules = granted_rules(
            &[second, again, first],
            &turn(&setting, Some(setting.task), TurnId::generate()),
        );
        assert_eq!(
            rules,
            [command("npm test").unwrap(), domain("docs.rs").unwrap()]
        );
    }
}
