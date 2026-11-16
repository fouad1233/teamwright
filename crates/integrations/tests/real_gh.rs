//! Reads a real repository through the `gh` of this computer. Ignored by
//! default: it needs `gh` installed, signed in, and the network. Run it with
//! `cargo test -p teamwright-integrations --test real_gh -- --ignored`, and
//! name another repository in `TEAMWRIGHT_GH_REPOSITORY` if you like. It
//! only reads; it creates nothing.

use teamwright_application::{GitHubProgram, GitHubSignIn, ItemKind};
use teamwright_domain::RepositoryName;
use teamwright_integrations::{GhCommandLine, GhLocator};

#[test]
#[ignore = "needs gh signed in and the network"]
fn the_pull_requests_and_issues_of_a_real_repository_are_read() {
    let gh = GhCommandLine::new(GhLocator::of_this_computer());
    let GitHubSignIn::SignedIn { program, account } = gh.sign_in(None) else {
        panic!("gh is not installed or not signed in");
    };
    println!("gh at {}, signed in as {account:?}", program.display());

    let name = std::env::var("TEAMWRIGHT_GH_REPOSITORY")
        .unwrap_or_else(|_| "fouad1233/teamwright".to_owned());
    let repository = RepositoryName::parse(&name).unwrap();
    gh.check_repository(&program, &repository).unwrap();
    let pulls = gh.pull_requests(&program, &repository).unwrap();
    let issues = gh.issues(&program, &repository).unwrap();
    println!(
        "{repository}: {} pull requests, {} issues",
        pulls.len(),
        issues.len()
    );
    for item in pulls.iter().chain(&issues).take(5) {
        println!(
            "  #{} {:?} {:?} {}",
            item.number, item.kind, item.state, item.title
        );
    }
    assert!(pulls.iter().all(|item| item.kind == ItemKind::PullRequest));
    assert!(issues.iter().all(|item| item.kind == ItemKind::Issue));

    let missing = RepositoryName::parse("fouad1233/no-such-repository-of-teamwright").unwrap();
    let refused = gh.check_repository(&program, &missing).unwrap_err();
    println!("a missing repository: {refused}");
}
