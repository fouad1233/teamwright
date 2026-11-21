//! The skills and connectors a manager gives the specialists
//! ([decision 0039](../../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md)).
//!
//! A project has a library: the skills and the connectors the manager has
//! added. Each specialist has a kit: the skills and the connectors of the
//! library it was given, and of each connector the tools it may use.
//!
//! - A **skill** is a folder with a `SKILL.md` that teaches the agent
//!   program how to do a kind of work. [`SkillFile::check`] is the rule
//!   about what that file may say.
//! - A **connector** is a tool server that gives a session tools reaching
//!   something outside its folder. [`Connector`] is how it is started and
//!   which tools it has. It never holds a secret: a token is named by the
//!   environment variable that holds it on the computer that runs the turn.
//!
//! The rules here are about one value. That a specialist is given only
//! tools its connector has, and only things of its own project, are rules
//! of the use cases.

use std::collections::BTreeSet;
use std::fmt;

use crate::error::DomainError;
use crate::ids::{ConnectorId, MemberId, ProjectId, SkillId};
use crate::time::Timestamp;

/// The built-in tools a skill may ask to use without asking: the ones a
/// specialist may use without asking anyway. A skill that asks for more
/// would widen what the manager chose ([decision 0011]).
///
/// [decision 0011]: ../../../docs/decisions/0011-what-a-specialist-may-do.md
pub const TOOLS_A_SKILL_MAY_ASK_FOR: [&str; 3] = ["Read", "Glob", "Grep"];

/// The longest description of a skill, as the open format of skills has it.
pub const MAX_SKILL_DESCRIPTION: usize = 1024;

/// The most tools a connector may list.
pub const MAX_TOOLS: usize = 64;

/// The most arguments of the program that starts a connector.
pub const MAX_ARGUMENTS: usize = 32;

/// The most environment variables or headers of a connector.
pub const MAX_SECRETS: usize = 8;

/// The name of the tool server of the team itself, which no connector may
/// take.
const TEAM_BUS_NAME: &str = "teamwright";

/// Names the agent program keeps for tool servers of its own.
const RESERVED_SERVER_NAMES: [&str; 4] = [
    TEAM_BUS_NAME,
    "workspace",
    "claude-in-chrome",
    "computer-use",
];

/// The words that make a parameter of a web address look like a secret.
const SECRET_WORDS: [&str; 6] = ["token", "key", "secret", "password", "auth", "signature"];

fn not_a_skill(reason: impl Into<String>) -> DomainError {
    DomainError::NotASkill {
        reason: reason.into(),
    }
}

fn invalid_connector(reason: impl Into<String>) -> DomainError {
    DomainError::InvalidConnector {
        reason: reason.into(),
    }
}

/// Whether a text is a lower-case name of letters, digits and single
/// hyphens, not starting or ending with a hyphen, of at most `max`
/// characters.
fn is_lower_case_name(text: &str, max: usize) -> bool {
    !text.is_empty()
        && text.len() <= max
        && text.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && !text.starts_with('-')
        && !text.ends_with('-')
        && !text.contains("--")
}

/// The name of a skill: lower-case letters, digits and hyphens, at most 64
/// characters, as the open format of skills asks. The agent program knows
/// the skill by it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SkillName(String);

impl SkillName {
    /// The longest name.
    pub const MAX_LENGTH: usize = 64;

    /// Checks a name.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::NotASkill`] if the name does not have the
    /// form, or is one the agent program keeps for itself.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let text = text.trim();
        if !is_lower_case_name(text, Self::MAX_LENGTH) {
            return Err(not_a_skill(format!(
                "its name \"{text}\" is not lower-case letters, digits and single hyphens, at \
                 most {} characters",
                Self::MAX_LENGTH
            )));
        }
        if text == "synced" || text.starts_with("anthropic-skills") {
            return Err(not_a_skill(format!(
                "its name \"{text}\" is kept by Claude Code for skills of its own"
            )));
        }
        Ok(Self(text.to_owned()))
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SkillName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// What the `SKILL.md` of a folder says, checked: its name and its
/// description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillFile {
    name: SkillName,
    description: String,
}

/// One key of the front matter, with the text after its colon and the
/// lines that continue it.
#[derive(Debug)]
struct FrontEntry<'text> {
    key: &'text str,
    value: &'text str,
    more: Vec<&'text str>,
}

impl SkillFile {
    /// Checks the text of a `SKILL.md`, found in a folder of this name.
    ///
    /// The file starts with front matter between two lines of `---`, in the
    /// plain form skills use: `key: value` lines, a value continued on the
    /// lines below it, and lists. It needs a `description`. Its `name`, or
    /// else the name of the folder, is the name of the skill.
    ///
    /// A specialist runs no commands and uses only the tools the manager
    /// gave it ([decision 0011]). So a skill is refused if it would change
    /// that when it is used: if it has `hooks`, which run commands; if its
    /// `allowed-tools` name a tool other than `Read`, `Glob` and `Grep`; or
    /// if its text runs a command before it is read, with `` !`…` `` or a
    /// block that opens with ` ```! `.
    ///
    /// [decision 0011]: ../../../docs/decisions/0011-what-a-specialist-may-do.md
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::NotASkill`] with the reason.
    pub fn check(text: &str, folder_name: &str) -> Result<Self, DomainError> {
        let (entries, body) = front_matter(text)?;
        let entry = |key: &str| entries.iter().find(|entry| entry.key == key);

        if entry("hooks").is_some() {
            return Err(not_a_skill(
                "it has hooks, which run commands, and a specialist runs no commands",
            ));
        }
        if let Some(allowed) = entry("allowed-tools") {
            for tool in list_of(allowed) {
                if !TOOLS_A_SKILL_MAY_ASK_FOR.contains(&tool.as_str()) {
                    return Err(not_a_skill(format!(
                        "it asks to use {tool} without asking, and only the manager chooses \
                         what a specialist may use (a skill may ask for Read, Glob and Grep)"
                    )));
                }
            }
        }
        if runs_a_command(body) {
            return Err(not_a_skill(
                "it runs a command when it is used (with !` or ```!), and a specialist runs no \
                 commands",
            ));
        }

        let description = entry("description").map(scalar_of).unwrap_or_default();
        if description.is_empty() {
            return Err(not_a_skill(
                "its SKILL.md has no description, which says when the skill is used",
            ));
        }
        let length = description.chars().count();
        if length > MAX_SKILL_DESCRIPTION {
            return Err(not_a_skill(format!(
                "its description has {length} characters, and at most {MAX_SKILL_DESCRIPTION} \
                 are read"
            )));
        }
        let name = match entry("name").map(scalar_of) {
            Some(name) if !name.is_empty() => SkillName::parse(&name)?,
            _ => SkillName::parse(folder_name)?,
        };
        Ok(Self { name, description })
    }

    /// The name of the skill.
    #[must_use]
    pub const fn name(&self) -> &SkillName {
        &self.name
    }

    /// What the skill is for and when it is used, in one line.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// Splits a `SKILL.md` into the entries of its front matter and its body.
fn front_matter(text: &str) -> Result<(Vec<FrontEntry<'_>>, &str), DomainError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    let opening = lines.next().unwrap_or_default();
    if opening.trim_end() != "---" {
        return Err(not_a_skill(
            "its SKILL.md does not start with front matter between two lines of ---",
        ));
    }
    let mut consumed = opening.len();
    let mut entries: Vec<FrontEntry<'_>> = Vec::new();
    for line in lines {
        consumed += line.len();
        let content = line.trim_end();
        if content == "---" {
            return Ok((entries, &text[consumed..]));
        }
        let trimmed = content.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let continues =
            content.starts_with([' ', '\t']) || trimmed.starts_with("- ") || trimmed == "-";
        if continues {
            let Some(last) = entries.last_mut() else {
                return Err(not_a_skill(
                    "its front matter starts with a line that belongs to no key",
                ));
            };
            last.more.push(trimmed);
            continue;
        }
        let Some((key, value)) = content.split_once(':') else {
            return Err(not_a_skill(format!(
                "a line of its front matter is not of the form key: value: \"{}\"",
                shortened(content)
            )));
        };
        let key = key.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
        {
            return Err(not_a_skill(format!(
                "a line of its front matter is not of the form key: value: \"{}\"",
                shortened(content)
            )));
        }
        entries.push(FrontEntry {
            key,
            value: value.trim(),
            more: Vec::new(),
        });
    }
    Err(not_a_skill(
        "its front matter is not closed by a line of ---",
    ))
}

/// A line of a file, cut short for a message.
fn shortened(line: &str) -> String {
    let mut short: String = line.chars().take(60).collect();
    if line.chars().count() > 60 {
        short.push('…');
    }
    short
}

/// A value without the quotes around it.
fn unquoted(value: &str) -> &str {
    let value = value.trim();
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

/// The value of an entry as one line of text: a folded or literal block,
/// or a plain value continued on the lines below it, joined with spaces.
fn scalar_of(entry: &FrontEntry<'_>) -> String {
    let block = matches!(entry.value, ">" | "|" | ">-" | "|-" | ">+" | "|+");
    let mut parts: Vec<&str> = Vec::new();
    if !block && !entry.value.is_empty() {
        parts.push(entry.value);
    }
    parts.extend(entry.more.iter().copied());
    unquoted(&parts.join(" ")).trim().to_owned()
}

/// The value of an entry as a list: `[a, b]`, lines of `- a`, or words
/// separated by commas or spaces.
fn list_of(entry: &FrontEntry<'_>) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    let mut take = |text: &str| {
        let text = text.trim();
        let text = text
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
            .unwrap_or(text);
        // A rule such as `Bash(git *)` holds a space; it stays one item.
        let mut depth = 0_i32;
        let mut current = String::new();
        for character in text.chars() {
            match character {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            if depth <= 0 && (character == ',' || character.is_whitespace()) {
                if !current.is_empty() {
                    items.push(unquoted(&current).to_owned());
                    current.clear();
                }
            } else {
                current.push(character);
            }
        }
        if !current.is_empty() {
            items.push(unquoted(&current).to_owned());
        }
    };
    take(entry.value);
    for line in &entry.more {
        take(line.strip_prefix('-').unwrap_or(line));
    }
    items.retain(|item| !item.is_empty());
    items
}

/// Whether the body of a skill runs a command when the skill is used: an
/// inline `` !`…` `` at the start of a line or after a space, or a block
/// that opens with ` ```! `. This is how the agent program recognises them.
fn runs_a_command(body: &str) -> bool {
    body.lines().any(|line| {
        if line.trim_start().starts_with("```!") {
            return true;
        }
        let mut previous: Option<char> = None;
        let mut characters = line.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '!'
                && characters.peek() == Some(&'`')
                && previous.is_none_or(char::is_whitespace)
            {
                return true;
            }
            previous = Some(character);
        }
        false
    })
}

/// The plain data of a skill of the library, used to recreate one that was
/// stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSnapshot {
    /// The identifier of the skill.
    pub id: SkillId,
    /// The project whose library it is in.
    pub project_id: ProjectId,
    /// Its name.
    pub name: SkillName,
    /// What it is for, from its `SKILL.md`.
    pub description: String,
    /// When the manager added it.
    pub added_at: Timestamp,
}

/// A skill in the library of a project. The folder itself is kept on each
/// computer; this is what every computer knows of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    id: SkillId,
    project_id: ProjectId,
    name: SkillName,
    description: String,
    added_at: Timestamp,
}

impl Skill {
    /// A skill the manager adds to the library of a project, from the
    /// checked `SKILL.md` of its folder.
    #[must_use]
    pub fn add(project_id: ProjectId, file: SkillFile, now: Timestamp) -> Self {
        Self {
            id: SkillId::generate(),
            project_id,
            name: file.name,
            description: file.description,
            added_at: now,
        }
    }

    /// Recreates a stored skill.
    ///
    /// # Errors
    ///
    /// Returns an error if the description is empty or too long.
    pub fn restore(snapshot: SkillSnapshot) -> Result<Self, DomainError> {
        let length = snapshot.description.chars().count();
        if snapshot.description.trim().is_empty() || length > MAX_SKILL_DESCRIPTION {
            return Err(not_a_skill("its description is empty or too long"));
        }
        Ok(Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            name: snapshot.name,
            description: snapshot.description,
            added_at: snapshot.added_at,
        })
    }

    /// Takes the checked `SKILL.md` of a copy of the folder that was added
    /// again, on this computer or another one. The identifier stays, so
    /// the specialists that have the skill keep it.
    ///
    /// # Errors
    ///
    /// Returns an error if the file names another skill.
    pub fn take_again(&mut self, file: SkillFile) -> Result<(), DomainError> {
        if file.name != self.name {
            return Err(not_a_skill(format!(
                "it is the skill \"{}\", not \"{}\"",
                file.name, self.name
            )));
        }
        self.description = file.description;
        Ok(())
    }

    /// The identifier.
    #[must_use]
    pub const fn id(&self) -> SkillId {
        self.id
    }

    /// The project whose library it is in.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The name.
    #[must_use]
    pub const fn name(&self) -> &SkillName {
        &self.name
    }

    /// What it is for.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// When the manager added it.
    #[must_use]
    pub const fn added_at(&self) -> Timestamp {
        self.added_at
    }
}

/// The name of a connector: lower-case letters, digits and hyphens, at most
/// 40 characters. The tools of the connector are known to a session as
/// `mcp__<name>__<tool>`, so the name must be one the agent program writes
/// as it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectorName(String);

impl ConnectorName {
    /// The longest name.
    pub const MAX_LENGTH: usize = 40;

    /// Checks a name.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidConnector`] if the name does not have
    /// the form, or belongs to a tool server of the team or of Claude Code.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let text = text.trim();
        if !is_lower_case_name(text, Self::MAX_LENGTH) {
            return Err(invalid_connector(format!(
                "the name of a connector is lower-case letters, digits and single hyphens, at \
                 most {} characters, such as parts-catalog",
                Self::MAX_LENGTH
            )));
        }
        if RESERVED_SERVER_NAMES.contains(&text) {
            return Err(invalid_connector(format!(
                "the name \"{text}\" belongs to a tool server of Teamwright or Claude Code"
            )));
        }
        Ok(Self(text.to_owned()))
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConnectorName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The name of a tool a connector offers, as the connector names it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolName(String);

impl ToolName {
    /// The longest name.
    pub const MAX_LENGTH: usize = 64;

    /// Checks a name: letters, digits, `_` and `-`, not starting with `-`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidConnector`] if the name does not have
    /// the form.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let text = text.trim();
        let fits = !text.is_empty()
            && text.len() <= Self::MAX_LENGTH
            && !text.starts_with('-')
            && text
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "_-".contains(character));
        if !fits {
            return Err(invalid_connector(format!(
                "\"{}\" is not the name of a tool: letters, digits, _ and -, at most {} \
                 characters",
                shortened(text),
                Self::MAX_LENGTH
            )));
        }
        Ok(Self(text.to_owned()))
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The name of an environment variable that holds a secret on the computer
/// that runs a turn. Teamwright keeps the name, never the value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VariableName(String);

impl VariableName {
    /// The longest name.
    pub const MAX_LENGTH: usize = 64;

    /// Checks a name: a letter or `_`, then letters, digits and `_`.
    ///
    /// The variables of the agent program itself, whose names start with
    /// `CLAUDE` or `ANTHROPIC`, cannot be handed to a connector: they hold
    /// the manager's own access to it.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidConnector`] if the name does not have
    /// the form or is one of those.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let text = text.trim();
        let fits = !text.is_empty()
            && text.len() <= Self::MAX_LENGTH
            && text
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
            && text
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_');
        if !fits {
            return Err(invalid_connector(format!(
                "\"{}\" is not the name of an environment variable, such as PARTS_TOKEN",
                shortened(text)
            )));
        }
        let upper = text.to_ascii_uppercase();
        if upper.starts_with("CLAUDE") || upper.starts_with("ANTHROPIC") {
            return Err(invalid_connector(format!(
                "the variable {text} belongs to Claude Code and is never handed to a connector"
            )));
        }
        Ok(Self(text.to_owned()))
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VariableName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A header a connector at a web address is sent, whose value is the
/// secret in an environment variable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretHeader {
    name: String,
    variable: VariableName,
    bearer: bool,
}

impl SecretHeader {
    /// Checks a header: its name is letters, digits and `-`. With `bearer`,
    /// the value is `Bearer ` and the secret, as the header
    /// `Authorization` usually wants it.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidConnector`] if the name does not have
    /// the form.
    pub fn new(name: &str, variable: VariableName, bearer: bool) -> Result<Self, DomainError> {
        let name = name.trim();
        let fits = !name.is_empty()
            && name.len() <= 64
            && name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-');
        if !fits {
            return Err(invalid_connector(format!(
                "\"{}\" is not the name of a header, such as Authorization",
                shortened(name)
            )));
        }
        Ok(Self {
            name: name.to_owned(),
            variable,
            bearer,
        })
    }

    /// The name of the header.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The variable that holds its secret.
    #[must_use]
    pub const fn variable(&self) -> &VariableName {
        &self.variable
    }

    /// Whether the value is `Bearer ` and the secret.
    #[must_use]
    pub const fn bearer(&self) -> bool {
        self.bearer
    }
}

/// How a connector is started.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConnectorStart {
    /// A program on the computer that runs the turn, which the agent
    /// program starts and talks to through its input and output.
    Program {
        /// The program, by name or by path.
        program: String,
        /// Its arguments.
        arguments: Vec<String>,
        /// The environment variables it is handed, by name. Their values
        /// come from the environment Teamwright was started in.
        variables: Vec<VariableName>,
    },
    /// A tool server at a web address.
    Address {
        /// The address, `https://`, or `http://` on this computer itself.
        url: String,
        /// The headers that carry its secrets.
        headers: Vec<SecretHeader>,
    },
}

impl ConnectorStart {
    /// A connector started as a program.
    ///
    /// Neither the program nor an argument may contain `$`: the agent
    /// program would replace `${NAME}` there with a variable of its own
    /// environment, and so hand a connector a secret nobody chose for it.
    /// Secrets reach a program only through the variables named here.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidConnector`] if the program is empty,
    /// starts with `-`, or if the program or an argument contains `$` or a
    /// line break, or if there are too many arguments or variables, or a
    /// variable is named twice.
    pub fn program(
        program: &str,
        arguments: Vec<String>,
        variables: Vec<VariableName>,
    ) -> Result<Self, DomainError> {
        let program = program.trim();
        if program.is_empty() {
            return Err(invalid_connector(
                "a connector started as a program names the program",
            ));
        }
        if program.starts_with('-') {
            return Err(invalid_connector(
                "the program of a connector cannot start with -",
            ));
        }
        if arguments.len() > MAX_ARGUMENTS {
            return Err(invalid_connector(format!(
                "a connector has at most {MAX_ARGUMENTS} arguments"
            )));
        }
        for text in std::iter::once(program).chain(arguments.iter().map(String::as_str)) {
            if text.contains('$') {
                return Err(invalid_connector(
                    "the program and its arguments cannot contain $; name the environment \
                     variables the connector needs instead",
                ));
            }
            if text.chars().any(char::is_control) || text.len() > 1024 {
                return Err(invalid_connector(
                    "the program and each argument are one line of at most 1024 characters",
                ));
            }
        }
        check_secret_count(variables.len())?;
        let distinct: BTreeSet<&VariableName> = variables.iter().collect();
        if distinct.len() != variables.len() {
            return Err(invalid_connector("a variable is named twice"));
        }
        Ok(Self::Program {
            program: program.to_owned(),
            arguments,
            variables,
        })
    }

    /// A connector at a web address.
    ///
    /// The address is kept with the connector and travels to the other
    /// computers of the project, so it must not carry a secret: it may not
    /// have a user and a password, a parameter whose name says token, key,
    /// secret, password, auth or signature, or a `$`. A secret goes in a
    /// header, from an environment variable.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidConnector`]. The message never repeats
    /// the address.
    pub fn address(url: &str, headers: Vec<SecretHeader>) -> Result<Self, DomainError> {
        let url = url.trim();
        check_address(url)?;
        check_secret_count(headers.len())?;
        let distinct: BTreeSet<String> = headers
            .iter()
            .map(|header| header.name.to_ascii_lowercase())
            .collect();
        if distinct.len() != headers.len() {
            return Err(invalid_connector("a header is named twice"));
        }
        Ok(Self::Address {
            url: url.to_owned(),
            headers,
        })
    }

    /// The environment variables a turn needs for this connector.
    #[must_use]
    pub fn variables(&self) -> Vec<&VariableName> {
        match self {
            Self::Program { variables, .. } => variables.iter().collect(),
            Self::Address { headers, .. } => headers.iter().map(SecretHeader::variable).collect(),
        }
    }
}

fn check_secret_count(count: usize) -> Result<(), DomainError> {
    if count > MAX_SECRETS {
        return Err(invalid_connector(format!(
            "a connector has at most {MAX_SECRETS} variables or headers"
        )));
    }
    Ok(())
}

/// Checks a web address of a connector without ever repeating it.
fn check_address(url: &str) -> Result<(), DomainError> {
    let refuse = |why: &str| {
        Err(invalid_connector(format!(
            "the address of a connector {why}"
        )))
    };
    if url.len() > 2048
        || url
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return refuse("is one line without spaces, of at most 2048 characters");
    }
    if url.contains('$') {
        return refuse("cannot contain $; a secret goes in a header, from an environment variable");
    }
    let rest = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let host = host.rsplit_once(':').map_or(host, |(name, port)| {
            if port.chars().all(|character| character.is_ascii_digit()) && !name.ends_with(':') {
                name
            } else {
                host
            }
        });
        if !matches!(host, "localhost" | "127.0.0.1" | "[::1]") {
            return refuse("starts with https://, or with http:// only on this computer itself");
        }
        rest
    } else {
        return refuse("starts with https://");
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() {
        return refuse("names a host");
    }
    if authority.contains('@') {
        return refuse(
            "cannot carry a user or a password; a secret goes in a header, from an environment \
             variable",
        );
    }
    if let Some((_, query)) = rest.split_once('?') {
        let query = query.split('#').next().unwrap_or_default();
        for parameter in query.split('&') {
            let name = parameter
                .split('=')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if SECRET_WORDS.iter().any(|word| name.contains(word)) {
                return refuse(
                    "cannot carry a secret in its parameters; a secret goes in a header, from \
                     an environment variable",
                );
            }
        }
    }
    Ok(())
}

/// The plain data of a connector, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorSnapshot {
    /// The identifier of the connector.
    pub id: ConnectorId,
    /// The project whose library it is in.
    pub project_id: ProjectId,
    /// Its name.
    pub name: ConnectorName,
    /// How it is started.
    pub start: ConnectorStart,
    /// The tools it offers that the manager may give.
    pub tools: Vec<ToolName>,
    /// When the manager added it.
    pub added_at: Timestamp,
}

/// A connector in the library of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connector {
    id: ConnectorId,
    project_id: ProjectId,
    name: ConnectorName,
    start: ConnectorStart,
    tools: Vec<ToolName>,
    added_at: Timestamp,
}

impl Connector {
    /// A connector the manager adds to the library of a project.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidConnector`] if it lists no tool, too
    /// many, or one twice.
    pub fn add(
        project_id: ProjectId,
        name: ConnectorName,
        start: ConnectorStart,
        tools: Vec<ToolName>,
        now: Timestamp,
    ) -> Result<Self, DomainError> {
        check_tools(&tools)?;
        Ok(Self {
            id: ConnectorId::generate(),
            project_id,
            name,
            start,
            tools,
            added_at: now,
        })
    }

    /// Recreates a stored connector.
    ///
    /// # Errors
    ///
    /// Returns an error if its tools break the rules of [`Self::add`].
    pub fn restore(snapshot: ConnectorSnapshot) -> Result<Self, DomainError> {
        check_tools(&snapshot.tools)?;
        Ok(Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            name: snapshot.name,
            start: snapshot.start,
            tools: snapshot.tools,
            added_at: snapshot.added_at,
        })
    }

    /// The identifier.
    #[must_use]
    pub const fn id(&self) -> ConnectorId {
        self.id
    }

    /// The project whose library it is in.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The name.
    #[must_use]
    pub const fn name(&self) -> &ConnectorName {
        &self.name
    }

    /// How it is started.
    #[must_use]
    pub const fn start(&self) -> &ConnectorStart {
        &self.start
    }

    /// The tools it offers that the manager may give.
    #[must_use]
    pub fn tools(&self) -> &[ToolName] {
        &self.tools
    }

    /// When the manager added it.
    #[must_use]
    pub const fn added_at(&self) -> Timestamp {
        self.added_at
    }
}

fn check_tools(tools: &[ToolName]) -> Result<(), DomainError> {
    if tools.is_empty() {
        return Err(invalid_connector(
            "a connector lists at least one of its tools, so that a specialist can be given it",
        ));
    }
    if tools.len() > MAX_TOOLS {
        return Err(invalid_connector(format!(
            "a connector lists at most {MAX_TOOLS} tools"
        )));
    }
    let distinct: BTreeSet<&ToolName> = tools.iter().collect();
    if distinct.len() != tools.len() {
        return Err(invalid_connector("a tool is listed twice"));
    }
    Ok(())
}

/// A connector as one specialist is given it: the connector and the tools
/// of it that the specialist may use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GivenConnector {
    connector_id: ConnectorId,
    tools: Vec<ToolName>,
}

impl GivenConnector {
    /// Gives a connector with these of its tools.
    ///
    /// A connector is never given whole: the tools are named one by one,
    /// and a connector given no tool is not given at all.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidConnector`] if no tool is named, or
    /// one twice.
    pub fn new(connector_id: ConnectorId, tools: Vec<ToolName>) -> Result<Self, DomainError> {
        if tools.is_empty() {
            return Err(invalid_connector(
                "a specialist is given a connector with at least one of its tools",
            ));
        }
        let distinct: BTreeSet<&ToolName> = tools.iter().collect();
        if distinct.len() != tools.len() {
            return Err(invalid_connector("a tool is given twice"));
        }
        Ok(Self {
            connector_id,
            tools,
        })
    }

    /// The connector.
    #[must_use]
    pub const fn connector_id(&self) -> ConnectorId {
        self.connector_id
    }

    /// The tools of it the specialist may use.
    #[must_use]
    pub fn tools(&self) -> &[ToolName] {
        &self.tools
    }
}

/// What one specialist was given of the library of its project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberKit {
    member_id: MemberId,
    project_id: ProjectId,
    skills: Vec<SkillId>,
    connectors: Vec<GivenConnector>,
}

impl MemberKit {
    /// The kit of a specialist that was given nothing.
    #[must_use]
    pub const fn empty(member_id: MemberId, project_id: ProjectId) -> Self {
        Self {
            member_id,
            project_id,
            skills: Vec::new(),
            connectors: Vec::new(),
        }
    }

    /// A kit with these skills and connectors.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidConnector`] if a skill or a connector
    /// is given twice.
    pub fn new(
        member_id: MemberId,
        project_id: ProjectId,
        skills: Vec<SkillId>,
        connectors: Vec<GivenConnector>,
    ) -> Result<Self, DomainError> {
        let distinct_skills: BTreeSet<&SkillId> = skills.iter().collect();
        if distinct_skills.len() != skills.len() {
            return Err(invalid_connector("a skill is given twice"));
        }
        let distinct_connectors: BTreeSet<ConnectorId> = connectors
            .iter()
            .map(GivenConnector::connector_id)
            .collect();
        if distinct_connectors.len() != connectors.len() {
            return Err(invalid_connector("a connector is given twice"));
        }
        Ok(Self {
            member_id,
            project_id,
            skills,
            connectors,
        })
    }

    /// The specialist.
    #[must_use]
    pub const fn member_id(&self) -> MemberId {
        self.member_id
    }

    /// Its project.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The skills it was given.
    #[must_use]
    pub fn skills(&self) -> &[SkillId] {
        &self.skills
    }

    /// The connectors it was given, each with its tools.
    #[must_use]
    pub fn connectors(&self) -> &[GivenConnector] {
        &self.connectors
    }

    /// Whether it was given nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty() && self.connectors.is_empty()
    }

    /// Takes a skill out, as when it leaves the library. Says whether the
    /// kit had it.
    pub fn without_skill(&mut self, skill: SkillId) -> bool {
        let before = self.skills.len();
        self.skills.retain(|kept| *kept != skill);
        before != self.skills.len()
    }

    /// Takes a connector out, as when it leaves the library. Says whether
    /// the kit had it.
    pub fn without_connector(&mut self, connector: ConnectorId) -> bool {
        let before = self.connectors.len();
        self.connectors
            .retain(|given| given.connector_id != connector);
        before != self.connectors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checked(text: &str) -> Result<SkillFile, DomainError> {
        SkillFile::check(text, "pcb-review")
    }

    fn reason(error: DomainError) -> String {
        match error {
            DomainError::NotASkill { reason } | DomainError::InvalidConnector { reason } => reason,
            other => panic!("unexpected error {other:?}"),
        }
    }

    #[test]
    fn a_skill_with_a_name_and_a_description_is_one() {
        let file = checked(
            "---\nname: pcb-review\ndescription: Reviews a board layout. Use when a layout \
             is ready.\n---\n\n# Review\nLook at the layers.\n",
        )
        .unwrap();
        assert_eq!(file.name().as_str(), "pcb-review");
        assert_eq!(
            file.description(),
            "Reviews a board layout. Use when a layout is ready."
        );
    }

    #[test]
    fn without_a_name_the_folder_names_the_skill() {
        let file = SkillFile::check(
            "---\ndescription: \"Quoted, with: a colon\"\n---\nBody",
            "bom-check",
        )
        .unwrap();
        assert_eq!(file.name().as_str(), "bom-check");
        assert_eq!(file.description(), "Quoted, with: a colon");
    }

    #[test]
    fn a_description_may_be_a_block_or_go_on_below() {
        let folded = checked("---\ndescription: >\n  Reviews a board\n  layout.\n---\n").unwrap();
        assert_eq!(folded.description(), "Reviews a board layout.");
        let continued =
            checked("\u{feff}---\r\ndescription: Reviews\r\n  a layout.\r\n---\r\n").unwrap();
        assert_eq!(continued.description(), "Reviews a layout.");
    }

    #[test]
    fn a_skill_without_front_matter_or_description_is_refused() {
        assert!(reason(checked("# Review\n").unwrap_err()).contains("front matter"));
        assert!(reason(checked("---\nname: x\n").unwrap_err()).contains("not closed"));
        assert!(
            reason(checked("---\nname: pcb-review\n---\n").unwrap_err()).contains("description")
        );
        assert!(reason(checked("---\njust words\n---\n").unwrap_err()).contains("key: value"));
        let long = format!(
            "---\ndescription: {}\n---\n",
            "a".repeat(MAX_SKILL_DESCRIPTION + 1)
        );
        assert!(reason(checked(&long).unwrap_err()).contains("1024"));
    }

    #[test]
    fn a_name_has_the_form_of_the_open_format() {
        assert!(checked("---\nname: PCB Review\ndescription: x\n---\n").is_err());
        assert!(SkillFile::check("---\ndescription: x\n---\n", "My Skill").is_err());
        assert!(SkillName::parse("synced").is_err());
        assert!(SkillName::parse("anthropic-skills-x").is_err());
        assert!(SkillName::parse("a--b").is_err());
        assert!(SkillName::parse(&"a".repeat(65)).is_err());
        assert!(SkillName::parse("review-2").is_ok());
    }

    #[test]
    fn a_skill_that_runs_commands_is_refused() {
        let hooks = "---\ndescription: x\nhooks:\n  PreToolUse:\n    - command: rm\n---\n";
        assert!(reason(checked(hooks).unwrap_err()).contains("hooks"));
        let inline = "---\ndescription: x\n---\nThe diff: !`git diff`\n";
        assert!(reason(checked(inline).unwrap_err()).contains("runs a command"));
        let at_start = "---\ndescription: x\n---\n!`date`\n";
        assert!(checked(at_start).is_err());
        let fenced = "---\ndescription: x\n---\n```!\nnode --version\n```\n";
        assert!(checked(fenced).is_err());
        // Not at the start of a line or after a space: the program does
        // not run it, and neither is the skill refused.
        let glued = "---\ndescription: x\n---\nKEY=!`cmd` and a plain `code` span!\n";
        assert!(checked(glued).is_ok());
    }

    #[test]
    fn a_skill_may_ask_only_for_the_tools_a_specialist_has() {
        assert!(checked("---\ndescription: x\nallowed-tools: Read Grep\n---\n").is_ok());
        assert!(checked("---\ndescription: x\nallowed-tools: [Read, Glob]\n---\n").is_ok());
        assert!(checked("---\ndescription: x\nallowed-tools:\n  - Read\n  - Grep\n---\n").is_ok());
        let bash = "---\ndescription: x\nallowed-tools: Read Bash(git status *)\n---\n";
        assert!(reason(checked(bash).unwrap_err()).contains("Bash(git status *)"));
        let tool = "---\ndescription: x\nallowed-tools:\n  - mcp__parts__order\n---\n";
        assert!(reason(checked(tool).unwrap_err()).contains("mcp__parts__order"));
    }

    #[test]
    fn a_connector_name_is_one_the_program_writes_as_it_is() {
        assert!(ConnectorName::parse("parts-catalog").is_ok());
        assert!(ConnectorName::parse("Parts").is_err());
        assert!(ConnectorName::parse("parts_catalog").is_err());
        assert!(ConnectorName::parse("my.server").is_err());
        assert!(ConnectorName::parse("teamwright").is_err());
        assert!(ConnectorName::parse("workspace").is_err());
        assert!(ConnectorName::parse(&"a".repeat(41)).is_err());
    }

    #[test]
    fn tools_and_variables_have_their_forms() {
        assert!(ToolName::parse("search_parts").is_ok());
        assert!(ToolName::parse("get-part").is_ok());
        assert!(ToolName::parse("mcp tool").is_err());
        assert!(ToolName::parse("-x").is_err());
        assert!(VariableName::parse("PARTS_TOKEN").is_ok());
        assert!(VariableName::parse("1TOKEN").is_err());
        assert!(VariableName::parse("A-B").is_err());
        assert!(
            reason(VariableName::parse("ANTHROPIC_API_KEY").unwrap_err()).contains("Claude Code")
        );
        assert!(VariableName::parse("claude_code_oauth_token").is_err());
    }

    fn variable(name: &str) -> VariableName {
        VariableName::parse(name).unwrap()
    }

    #[test]
    fn a_program_hands_secrets_only_through_named_variables() {
        let start = ConnectorStart::program(
            "npx",
            vec!["-y".to_owned(), "parts-server".to_owned()],
            vec![variable("PARTS_TOKEN")],
        )
        .unwrap();
        assert_eq!(start.variables(), [&variable("PARTS_TOKEN")]);
        let dollar = ConnectorStart::program(
            "server",
            vec!["--token=${ANTHROPIC_API_KEY}".to_owned()],
            Vec::new(),
        );
        assert!(reason(dollar.unwrap_err()).contains('$'));
        assert!(ConnectorStart::program("  ", Vec::new(), Vec::new()).is_err());
        assert!(ConnectorStart::program("--help", Vec::new(), Vec::new()).is_err());
        assert!(
            ConnectorStart::program("server", vec!["two\nlines".to_owned()], Vec::new()).is_err()
        );
        assert!(
            ConnectorStart::program("server", Vec::new(), vec![variable("A"), variable("A")])
                .is_err()
        );
    }

    #[test]
    fn an_address_never_carries_a_secret_and_is_never_repeated() {
        let header = SecretHeader::new("Authorization", variable("PARTS_TOKEN"), true).unwrap();
        let start = ConnectorStart::address("https://parts.example/mcp", vec![header]).unwrap();
        assert_eq!(start.variables(), [&variable("PARTS_TOKEN")]);
        assert!(ConnectorStart::address("http://localhost:8931/mcp", Vec::new()).is_ok());
        assert!(ConnectorStart::address("http://127.0.0.1/mcp", Vec::new()).is_ok());
        for refused in [
            "http://parts.example/mcp",
            "ftp://parts.example",
            "https://user:hunter2@parts.example/mcp",
            "https://parts.example/mcp?api_key=hunter2",
            "https://parts.example/mcp?access_token=hunter2",
            "https://parts.example/${PARTS_TOKEN}",
            "https:// parts.example",
            "https://",
        ] {
            let error = ConnectorStart::address(refused, Vec::new()).unwrap_err();
            assert!(!reason(error).contains("hunter2"), "{refused}");
        }
        assert!(ConnectorStart::address("https://parts.example/mcp?region=eu", Vec::new()).is_ok());
        assert!(SecretHeader::new("X Key", variable("A"), false).is_err());
        let twice = vec![
            SecretHeader::new("X-Key", variable("A"), false).unwrap(),
            SecretHeader::new("x-key", variable("B"), false).unwrap(),
        ];
        assert!(ConnectorStart::address("https://parts.example", twice).is_err());
    }

    fn tool(name: &str) -> ToolName {
        ToolName::parse(name).unwrap()
    }

    #[test]
    fn a_connector_lists_its_tools_once_each() {
        let start = ConnectorStart::program("parts-server", Vec::new(), Vec::new()).unwrap();
        let name = ConnectorName::parse("parts").unwrap();
        let project = ProjectId::generate();
        let now = chrono::Utc::now();
        assert!(Connector::add(project, name.clone(), start.clone(), Vec::new(), now).is_err());
        assert!(
            Connector::add(
                project,
                name.clone(),
                start.clone(),
                vec![tool("a"), tool("a")],
                now
            )
            .is_err()
        );
        let connector = Connector::add(project, name, start, vec![tool("search")], now).unwrap();
        assert_eq!(connector.tools(), [tool("search")]);
    }

    #[test]
    fn a_kit_gives_each_thing_once_and_a_connector_tool_by_tool() {
        let connector = ConnectorId::generate();
        assert!(GivenConnector::new(connector, Vec::new()).is_err());
        assert!(GivenConnector::new(connector, vec![tool("a"), tool("a")]).is_err());
        let given = GivenConnector::new(connector, vec![tool("search")]).unwrap();
        let skill = SkillId::generate();
        let member = MemberId::generate();
        let project = ProjectId::generate();
        assert!(MemberKit::new(member, project, vec![skill, skill], Vec::new()).is_err());
        assert!(
            MemberKit::new(
                member,
                project,
                Vec::new(),
                vec![given.clone(), given.clone()]
            )
            .is_err()
        );
        let mut kit = MemberKit::new(member, project, vec![skill], vec![given]).unwrap();
        assert!(kit.without_skill(skill));
        assert!(!kit.without_skill(skill));
        assert!(kit.without_connector(connector));
        assert!(kit.is_empty());
    }

    #[test]
    fn a_skill_added_again_must_be_the_same_skill() {
        let project = ProjectId::generate();
        let first = checked("---\ndescription: First.\n---\n").unwrap();
        let mut skill = Skill::add(project, first, chrono::Utc::now());
        let again = checked("---\ndescription: Second.\n---\n").unwrap();
        skill.take_again(again).unwrap();
        assert_eq!(skill.description(), "Second.");
        let other = SkillFile::check("---\ndescription: x\n---\n", "other").unwrap();
        assert!(skill.take_again(other).is_err());
    }
}
