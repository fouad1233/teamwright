//! GitHub through the manager's own `gh`. Teamwright keeps no token: `gh`
//! is signed in as the manager, and every call is one run of it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use teamwright_application::{GitHubItem, GitHubProgram, GitHubSignIn, ItemKind, ItemState};
use teamwright_domain::RepositoryName;

use crate::command::{self, Failure, Output};
use crate::locate::GhLocator;

/// How long one call of `gh` may take before it is stopped.
const TIME_LIMIT: Duration = Duration::from_secs(20);

/// How many pull requests, and how many issues, are read at most.
const MOST_ITEMS: &str = "200";

/// The manager's `gh`, found by a locator.
#[derive(Debug, Clone)]
pub struct GhCommandLine {
    locator: GhLocator,
}

impl GhCommandLine {
    /// The program, looked for where the locator looks.
    #[must_use]
    pub const fn new(locator: GhLocator) -> Self {
        Self { locator }
    }
}

/// A pull request as `gh pr list --json` writes it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequestJson {
    number: u64,
    title: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    head_ref_name: String,
    state: String,
    #[serde(default)]
    is_draft: bool,
    url: String,
}

/// An issue as `gh issue list --json` writes it.
#[derive(Debug, Deserialize)]
struct IssueJson {
    number: u64,
    title: String,
    state: String,
    url: String,
}

/// Reads the pull requests `gh` wrote.
fn pull_requests_of(json: &str) -> Result<Vec<GitHubItem>, String> {
    let read: Vec<PullRequestJson> = serde_json::from_str(json)
        .map_err(|_| "GitHub's command line program answered in a form it was not asked for")?;
    Ok(read
        .into_iter()
        .map(|pull| GitHubItem {
            kind: ItemKind::PullRequest,
            number: pull.number,
            title: pull.title,
            body: pull.body,
            branch: pull.head_ref_name,
            state: match pull.state.as_str() {
                "MERGED" => ItemState::Merged,
                "CLOSED" => ItemState::Closed,
                _ if pull.is_draft => ItemState::Draft,
                _ => ItemState::Open,
            },
            url: pull.url,
        })
        .collect())
}

/// Reads the issues `gh` wrote.
fn issues_of(json: &str) -> Result<Vec<GitHubItem>, String> {
    let read: Vec<IssueJson> = serde_json::from_str(json)
        .map_err(|_| "GitHub's command line program answered in a form it was not asked for")?;
    Ok(read
        .into_iter()
        .map(|issue| GitHubItem {
            kind: ItemKind::Issue,
            number: issue.number,
            title: issue.title,
            body: String::new(),
            branch: String::new(),
            state: if issue.state == "OPEN" {
                ItemState::Open
            } else {
                ItemState::Closed
            },
            url: issue.url,
        })
        .collect())
}

/// The account `gh auth status` says it is signed in as, if it says.
fn account_of(status: &str) -> Option<String> {
    let after = status.split("account ").nth(1)?;
    let account = after.split_whitespace().next()?;
    Some(account.to_owned())
}

/// The number of an issue from its address, which `gh issue create`
/// prints: `https://github.com/owner/name/issues/12`.
fn number_of(url: &str) -> Option<u64> {
    url.trim().rsplit('/').next()?.parse().ok()
}

/// Why a call failed, in plain words for the manager. The first line `gh`
/// wrote to its standard error says it best; it never holds a token.
fn failure_of(output: &Output, repository: Option<&RepositoryName>) -> String {
    let said = output
        .stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("it gave no reason");
    if said.contains("Could not resolve to a Repository") {
        return repository.map_or_else(
            || "GitHub knows no such repository".to_owned(),
            |repository| format!("GitHub knows no repository {repository} that you may see"),
        );
    }
    if said.contains("auth login") || said.contains("HTTP 401") {
        return "GitHub's command line program is not signed in: run `gh auth login`".to_owned();
    }
    let said: String = said.chars().take(200).collect();
    format!("GitHub's command line program failed: {said}")
}

fn run_failure(failure: &Failure) -> String {
    match failure {
        Failure::NotStarted(_) => "GitHub's command line program could not be started".to_owned(),
        Failure::TimedOut => format!(
            "GitHub did not answer within {} seconds",
            TIME_LIMIT.as_secs()
        ),
    }
}

/// Runs `gh` and answers with its standard output, or why it failed.
fn gh(
    program: &Path,
    arguments: &[&str],
    repository: Option<&RepositoryName>,
) -> Result<String, String> {
    let output = command::run(program, arguments, None, TIME_LIMIT)
        .map_err(|failure| run_failure(&failure))?;
    if output.succeeded {
        Ok(output.stdout)
    } else {
        Err(failure_of(&output, repository))
    }
}

impl GitHubProgram for GhCommandLine {
    fn sign_in(&self, named: Option<&Path>) -> GitHubSignIn {
        let Some(program) = self.locator.find(named) else {
            return GitHubSignIn::NotInstalled;
        };
        match command::run(
            &program,
            ["auth", "status", "--hostname", "github.com"],
            None,
            TIME_LIMIT,
        ) {
            Ok(output) if output.succeeded => GitHubSignIn::SignedIn {
                account: account_of(&format!("{}\n{}", output.stdout, output.stderr)),
                program,
            },
            Ok(_) => GitHubSignIn::NotSignedIn { program },
            Err(_) => GitHubSignIn::NotInstalled,
        }
    }

    fn remote_of(&self, folder: &Path) -> Option<String> {
        let folder_text = folder.to_string_lossy();
        let output = command::run(
            &PathBuf::from("git"),
            ["-C", folder_text.as_ref(), "remote", "get-url", "origin"],
            None,
            TIME_LIMIT,
        )
        .ok()?;
        let remote = output.stdout.trim();
        (output.succeeded && !remote.is_empty()).then(|| remote.to_owned())
    }

    fn check_repository(&self, program: &Path, repository: &RepositoryName) -> Result<(), String> {
        let name = repository.to_string();
        gh(
            program,
            &["repo", "view", &name, "--json", "nameWithOwner"],
            Some(repository),
        )
        .map(|_| ())
    }

    fn pull_requests(
        &self,
        program: &Path,
        repository: &RepositoryName,
    ) -> Result<Vec<GitHubItem>, String> {
        let name = repository.to_string();
        let json = gh(
            program,
            &[
                "pr",
                "list",
                "--repo",
                &name,
                "--state",
                "all",
                "--limit",
                MOST_ITEMS,
                "--json",
                "number,title,body,headRefName,state,isDraft,url",
            ],
            Some(repository),
        )?;
        pull_requests_of(&json)
    }

    fn issues(
        &self,
        program: &Path,
        repository: &RepositoryName,
    ) -> Result<Vec<GitHubItem>, String> {
        let name = repository.to_string();
        let json = gh(
            program,
            &[
                "issue",
                "list",
                "--repo",
                &name,
                "--state",
                "all",
                "--limit",
                MOST_ITEMS,
                "--json",
                "number,title,state,url",
            ],
            Some(repository),
        );
        match json {
            Ok(json) => issues_of(&json),
            // A repository may have its issues turned off: it then has none.
            Err(reason) if reason.contains("has disabled issues") => Ok(Vec::new()),
            Err(reason) => Err(reason),
        }
    }

    fn create_issue(
        &self,
        program: &Path,
        repository: &RepositoryName,
        title: &str,
        body: &str,
    ) -> Result<GitHubItem, String> {
        let name = repository.to_string();
        let printed = gh(
            program,
            &[
                "issue", "create", "--repo", &name, "--title", title, "--body", body,
            ],
            Some(repository),
        )?;
        let url = printed
            .lines()
            .map(str::trim)
            .rfind(|line| line.starts_with("https://"))
            .unwrap_or_default()
            .to_owned();
        Ok(GitHubItem {
            kind: ItemKind::Issue,
            number: number_of(&url).unwrap_or_default(),
            title: title.to_owned(),
            body: String::new(),
            branch: String::new(),
            state: ItemState::Open,
            url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pull_requests_are_read_with_their_state_and_branch() {
        let json = r#"[
            {"number":7,"title":"SH-4 Draw the schematic","body":"","headRefName":"sh-4-schematic",
             "state":"OPEN","isDraft":true,"url":"https://github.com/o/n/pull/7"},
            {"number":6,"title":"Route","body":"Closes SH-5","headRefName":"route",
             "state":"MERGED","isDraft":false,"url":"https://github.com/o/n/pull/6"},
            {"number":5,"title":"Old","headRefName":"old","state":"CLOSED","url":"https://github.com/o/n/pull/5"},
            {"number":4,"title":"Open","headRefName":"x","state":"OPEN","isDraft":false,"url":"https://github.com/o/n/pull/4"}
        ]"#;
        let pulls = pull_requests_of(json).unwrap();
        let states: Vec<ItemState> = pulls.iter().map(|pull| pull.state).collect();
        assert_eq!(
            states,
            [
                ItemState::Draft,
                ItemState::Merged,
                ItemState::Closed,
                ItemState::Open
            ]
        );
        assert_eq!(pulls[0].branch, "sh-4-schematic");
        assert_eq!(pulls[1].body, "Closes SH-5");
        assert!(pulls.iter().all(|pull| pull.kind == ItemKind::PullRequest));
    }

    #[test]
    fn issues_are_read_open_or_closed() {
        let json = r#"[{"number":3,"title":"SH-4 bug","state":"OPEN","url":"u3"},
                      {"number":2,"title":"x","state":"CLOSED","url":"u2"}]"#;
        let issues = issues_of(json).unwrap();
        assert_eq!(issues[0].state, ItemState::Open);
        assert_eq!(issues[1].state, ItemState::Closed);
        assert!(issues_of("not json").is_err());
    }

    #[test]
    fn the_account_and_the_number_of_a_new_issue_are_read() {
        let status = "github.com\n  ✓ Logged in to github.com account octo-cat (keyring)\n";
        assert_eq!(account_of(status).as_deref(), Some("octo-cat"));
        assert_eq!(account_of("nothing here"), None);
        assert_eq!(number_of("https://github.com/o/n/issues/12\n"), Some(12));
    }

    #[test]
    fn a_failure_is_said_in_plain_words() {
        let output = |stderr: &str| Output {
            succeeded: false,
            stdout: String::new(),
            stderr: stderr.to_owned(),
        };
        let repository = RepositoryName::parse("o/n").unwrap();
        assert_eq!(
            failure_of(
                &output("GraphQL: Could not resolve to a Repository with the name 'o/n'."),
                Some(&repository)
            ),
            "GitHub knows no repository o/n that you may see"
        );
        assert!(
            failure_of(&output("To get started, please run:  gh auth login"), None)
                .contains("not signed in")
        );
        assert_eq!(
            failure_of(&output("\n  something broke\nmore"), None),
            "GitHub's command line program failed: something broke"
        );
    }
}
