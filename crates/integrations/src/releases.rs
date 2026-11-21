//! The published releases of Teamwright, read from the public list of
//! GitHub ([decision 0043](../../../docs/decisions/0043-a-notice-of-newer-versions.md)).
//!
//! The request carries no token, no identity and nothing of the manager's
//! work: GitHub sees an address, and the name and version of the program in
//! the `User-Agent` header, which every request to its list must carry.

use std::time::Duration;

use serde::Deserialize;
use teamwright_application::ReleaseSource;
use teamwright_domain::{Release, Version, WebAddress};

/// The repository the releases are read from.
pub const REPOSITORY: &str = "fouad1233/teamwright";

/// How long a look may take, from the first try to the end of the answer.
/// It is short: nobody waits for it but a button.
const TIME_LIMIT: Duration = Duration::from_secs(8);

/// The most of an answer that is read. Thirty releases are far below it.
const MOST_ANSWER_BYTES: u64 = 2_000_000;

/// Reads the list of releases of a repository over HTTPS.
#[derive(Debug, Clone)]
pub struct GitHubReleases {
    /// The address of the list.
    list: String,
    /// The start of the address of one release, to which its tag is added.
    page: String,
}

impl GitHubReleases {
    /// The releases of Teamwright itself.
    #[must_use]
    pub fn of_teamwright() -> Self {
        Self::of_repository("https://api.github.com", REPOSITORY)
    }

    /// The releases of a repository, asked of an interface at `api`. A test
    /// points `api` at a listener on the same computer.
    #[must_use]
    pub fn of_repository(api: &str, repository: &str) -> Self {
        Self {
            list: format!("{api}/repos/{repository}/releases?per_page=30"),
            page: format!("https://github.com/{repository}/releases/tag/"),
        }
    }
}

/// What the list says of each release; the rest is not read.
#[derive(Debug, Deserialize)]
struct Entry {
    tag_name: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
}

/// The releases in an answer. A tag that is not a version is left out, and
/// so is a draft. A tag counts only if it is exactly the version as parsed,
/// with or without a `v` before it: `v1.0.0+anything` is refused, since
/// the part the version drops could steer the address somewhere else. The
/// address of a release is written from the repository and that checked
/// tag, which holds only numbers, letters, dots and one hyphen; the address
/// the answer gives is not used.
///
/// # Errors
///
/// If the answer is not a list of releases.
pub fn releases_in(body: &str, page: &str) -> Result<Vec<Release>, String> {
    let entries: Vec<Entry> = serde_json::from_str(body)
        .map_err(|_| "the list of releases was not understood".to_owned())?;
    Ok(entries
        .into_iter()
        .filter(|entry| !entry.draft)
        .filter_map(|entry| {
            let tag = entry.tag_name.trim();
            let version = Version::parse(tag)?;
            let canonical = version.text();
            if tag != canonical && tag.strip_prefix('v') != Some(canonical.as_str()) {
                return None;
            }
            let address = WebAddress::parse(&format!("{page}{tag}"), "release").ok()?;
            Some(Release {
                version,
                is_pre_release: entry.prerelease,
                address,
            })
        })
        .collect())
}

impl ReleaseSource for GitHubReleases {
    fn published_releases(&self) -> Result<Vec<Release>, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIME_LIMIT))
            .http_status_as_error(false)
            .user_agent(concat!("Teamwright/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        let mut response = agent
            .get(&self.list)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|error| match error {
                ureq::Error::Timeout(_) => "the list did not answer in time".to_owned(),
                ureq::Error::HostNotFound => "the host of the list was not found".to_owned(),
                _ => "the list could not be reached".to_owned(),
            })?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(format!("the list answered with status {status}"));
        }
        let body = response
            .body_mut()
            .with_config()
            .limit(MOST_ANSWER_BYTES)
            .read_to_string()
            .map_err(|_| "the answer of the list could not be read".to_owned())?;
        releases_in(&body, &self.page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "https://github.com/fouad1233/teamwright/releases/tag/";

    #[test]
    fn the_releases_of_an_answer_are_read_and_the_rest_is_left_out() {
        let body = r#"[
            {"tag_name": "v0.2.0", "prerelease": true, "draft": false, "html_url": "https://evil.example/x", "name": "two"},
            {"tag_name": "v0.1.0", "prerelease": true, "draft": false},
            {"tag_name": "nightly", "prerelease": false, "draft": false},
            {"tag_name": "v0.3.0", "prerelease": false, "draft": true},
            {"tag_name": "v1.0.0"}
        ]"#;
        let releases = releases_in(body, PAGE).unwrap();
        let tags: Vec<_> = releases.iter().map(|r| r.version.text()).collect();
        assert_eq!(tags, ["0.2.0", "0.1.0", "1.0.0"]);
        assert!(releases[0].is_pre_release && !releases[2].is_pre_release);
        assert_eq!(
            releases[0].address.as_str(),
            "https://github.com/fouad1233/teamwright/releases/tag/v0.2.0"
        );
    }

    #[test]
    fn a_tag_that_is_more_than_its_version_is_refused() {
        let body = r#"[
            {"tag_name": "v9.9.9+/%2e%2e/%2e%2e/x/y/releases/download/z/setup.exe"},
            {"tag_name": "v9.9.9+build5"},
            {"tag_name": "v1.0.0-RC.1"},
            {"tag_name": "vv1.0.0"},
            {"tag_name": "1.2.3"},
            {"tag_name": " v1.2.4 "},
            {"tag_name": "v1.3.0-rc.1"}
        ]"#;
        let releases = releases_in(body, PAGE).unwrap();
        let addresses: Vec<_> = releases.iter().map(|r| r.address.as_str()).collect();
        assert_eq!(
            addresses,
            [
                "https://github.com/fouad1233/teamwright/releases/tag/1.2.3",
                "https://github.com/fouad1233/teamwright/releases/tag/v1.2.4",
                "https://github.com/fouad1233/teamwright/releases/tag/v1.3.0-rc.1",
            ]
        );
    }

    #[test]
    fn an_answer_that_is_not_a_list_is_not_understood() {
        assert!(releases_in(r#"{"message": "rate limit"}"#, PAGE).is_err());
        assert!(releases_in("", PAGE).is_err());
    }
}
