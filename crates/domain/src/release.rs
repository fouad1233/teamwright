//! Versions of the program and the rule that says which published release,
//! if any, is newer than the one that runs
//! ([decision 0043](../../../docs/decisions/0043-a-notice-of-newer-versions.md)).

use std::cmp::Ordering;

use crate::integration::WebAddress;

/// A version such as `0.2.0` or `1.0.0-rc.1`: three numbers, and perhaps a
/// mark of a version before the release.
///
/// Versions are compared part by part as numbers (so `0.10.0` is newer than
/// `0.9.0`), never as text. A version with a mark comes before the same
/// numbers without it, as in semantic versioning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    numbers: [u64; 3],
    mark: Vec<MarkPart>,
}

/// One dot-separated part of the mark after the hyphen.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum MarkPart {
    // A number comes before a word, so the variants are in this order.
    Number(u64),
    Word(String),
}

impl Version {
    /// Reads a version or a tag name of one: `0.2.0`, `v0.2.0`,
    /// `v1.0.0-rc.1`. A part after `+` is ignored. Anything else, such as
    /// `nightly` or `v1.2`, is not a version.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let text = text.split('+').next().unwrap_or_default();
        let (core, mark) = match text.split_once('-') {
            Some((core, mark)) => (core, Some(mark)),
            None => (text, None),
        };
        let mut parts = core.split('.');
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        let numbers = [number(major)?, number(minor)?, number(patch)?];
        let mark = match mark {
            None => Vec::new(),
            Some(mark) => mark.split('.').map(mark_part).collect::<Option<Vec<_>>>()?,
        };
        Some(Self { numbers, mark })
    }

    /// Whether the version has a mark of a version before the release.
    #[must_use]
    pub fn has_mark(&self) -> bool {
        !self.mark.is_empty()
    }

    /// The version in words, without the `v`: `0.2.0` or `1.0.0-rc.1`.
    #[must_use]
    pub fn text(&self) -> String {
        let [major, minor, patch] = self.numbers;
        let mut text = format!("{major}.{minor}.{patch}");
        for (at, part) in self.mark.iter().enumerate() {
            text.push(if at == 0 { '-' } else { '.' });
            match part {
                MarkPart::Number(number) => text.push_str(&number.to_string()),
                MarkPart::Word(word) => text.push_str(word),
            }
        }
        text
    }
}

/// A part of a version that is only digits, as a number.
fn number(text: &str) -> Option<u64> {
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// One part of a mark: a number, or a word of letters and digits.
fn mark_part(text: &str) -> Option<MarkPart> {
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(number(text).map_or_else(
        || MarkPart::Word(text.to_ascii_lowercase()),
        MarkPart::Number,
    ))
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.numbers.cmp(&other.numbers).then_with(|| {
            match (self.mark.is_empty(), other.mark.is_empty()) {
                // No mark is the release itself: after every mark.
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.mark.cmp(&other.mark),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A release that was published: its version, whether it is a pre-release,
/// and the page where it can be had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The version the tag names.
    pub version: Version,
    /// Whether the publisher marked it a pre-release.
    pub is_pre_release: bool,
    /// The page of the release.
    pub address: WebAddress,
}

/// The release the manager should hear of, if there is one.
///
/// - A tag that is not a version is ignored: the caller passes only
///   releases whose tag read as a version.
/// - A full release is offered when it is newer than this program.
/// - A pre-release is offered only while this program is itself one (its
///   version has a mark, or a published pre-release has its version), or
///   while no full release has been published at all. A manager on a full
///   release is not sent to a pre-release; a manager on the first
///   pre-release hears of the next.
#[must_use]
pub fn newer_release<'a>(current: &Version, published: &'a [Release]) -> Option<&'a Release> {
    let is_pre_release_itself = current.has_mark()
        || published
            .iter()
            .any(|release| release.is_pre_release && release.version == *current);
    let has_full_release = published.iter().any(|release| !release.is_pre_release);
    let offers_pre_releases = is_pre_release_itself || !has_full_release;
    published
        .iter()
        .filter(|release| offers_pre_releases || !release.is_pre_release)
        .filter(|release| release.version > *current)
        .max_by(|a, b| a.version.cmp(&b.version))
}

/// Whether the version that the address of updates offers may be installed
/// over this program ([decision 0045](../../../docs/decisions/0045-installing-updates-from-inside.md)).
///
/// The updater asks this, not its own comparison, so that the notice and
/// the installer order versions by the same rule. Only a newer version is
/// installed, never the same or an older one. A version with a mark (a
/// release candidate) is installed only over a program that has a mark
/// itself: a program on a settled version is not moved onto an unsettled
/// one by itself, as the notice does not send it to one once full releases
/// exist.
#[must_use]
pub fn is_installable(current: &Version, offered: &Version) -> bool {
    offered > current && (!offered.has_mark() || current.has_mark())
}

/// How this copy of the program was installed, as far as updating it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallKind {
    /// An application bundle on macOS, from the `.dmg`.
    MacApplication,
    /// Installed on Windows by the setup program or the `.msi`.
    WindowsInstaller,
    /// An AppImage on Linux: one file, which the program may replace.
    AppImage,
    /// A package of the system on Linux, from the `.deb` or the `.rpm`.
    SystemPackage,
    /// Not installed by any installer: a copy built from the source.
    NotInstalled,
}

/// Why a copy of the program cannot install its own update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByHandReason {
    /// The files belong to the package manager of the system. Replacing
    /// them needs the administrator's password and would leave the package
    /// manager believing an older version is installed.
    SystemPackage,
    /// The copy was built from the source, not installed: there is no
    /// installed program to replace.
    NotInstalled,
}

impl InstallKind {
    /// Whether this copy installs its own updates, or why not.
    ///
    /// # Errors
    ///
    /// The reason an update has to be installed by hand.
    pub const fn updates_itself(self) -> Result<(), ByHandReason> {
        match self {
            Self::MacApplication | Self::WindowsInstaller | Self::AppImage => Ok(()),
            Self::SystemPackage => Err(ByHandReason::SystemPackage),
            Self::NotInstalled => Err(ByHandReason::NotInstalled),
        }
    }

    /// Whether the update is put in place only when the program restarts.
    /// On Windows the installer replaces the files of a program that is no
    /// longer running, so it runs as the program closes. Elsewhere the new
    /// version is put in place at once and starts at the next start.
    #[must_use]
    pub const fn installs_on_restart(self) -> bool {
        matches!(self, Self::WindowsInstaller)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn release(tag: &str, is_pre_release: bool) -> Release {
        Release {
            version: version(tag),
            is_pre_release,
            address: WebAddress::parse(
                &format!("https://github.com/o/r/releases/tag/{tag}"),
                "release",
            )
            .unwrap(),
        }
    }

    #[test]
    fn a_tag_with_or_without_v_is_a_version() {
        assert_eq!(version("v0.2.0"), version("0.2.0"));
        assert_eq!(version("v1.0.0-rc.1").text(), "1.0.0-rc.1");
        assert_eq!(version("1.2.3+build.5").text(), "1.2.3");
    }

    #[test]
    fn what_is_not_a_version_is_not_read() {
        for text in [
            "",
            "nightly",
            "v1.2",
            "1.2.3.4",
            "v1.x.0",
            "1.2.-3",
            "1.2.3-",
            "1.2.3-a..b",
            "latest",
        ] {
            assert_eq!(Version::parse(text), None, "{text}");
        }
    }

    #[test]
    fn versions_compare_as_numbers_not_as_text() {
        assert!(version("0.10.0") > version("0.9.0"));
        assert!(version("1.0.0") > version("0.99.99"));
        assert!(version("0.1.10") > version("0.1.9"));
        assert_eq!(version("v2.0.0"), version("2.0.0"));
    }

    #[test]
    fn a_mark_comes_before_the_release_and_is_ordered_part_by_part() {
        assert!(version("1.0.0-rc.1") < version("1.0.0"));
        assert!(version("1.0.0-rc.2") > version("1.0.0-rc.1"));
        assert!(version("1.0.0-rc.10") > version("1.0.0-rc.9"));
        assert!(version("1.0.0-rc") < version("1.0.0-rc.1"));
        assert!(version("1.0.0-beta") < version("1.0.0-rc"));
        assert!(version("1.0.0-1") < version("1.0.0-alpha"));
        assert!(version("1.0.0-rc.1") > version("0.9.0"));
    }

    #[test]
    fn nothing_newer_means_nothing_offered() {
        let published = [release("v0.1.0", true)];
        assert_eq!(newer_release(&version("0.1.0"), &published), None);
        assert_eq!(newer_release(&version("0.2.0"), &published), None);
        assert_eq!(newer_release(&version("0.1.0"), &[]), None);
    }

    #[test]
    fn the_newest_of_several_is_offered() {
        let published = [
            release("v0.2.0", false),
            release("v0.10.0", false),
            release("v0.9.0", false),
        ];
        let offered = newer_release(&version("0.1.0"), &published).unwrap();
        assert_eq!(offered.version.text(), "0.10.0");
    }

    #[test]
    fn a_pre_release_program_hears_of_the_next_pre_release() {
        // Today: 0.1.0 runs, and it is itself published as a pre-release.
        let published = [release("v0.1.0", true), release("v0.2.0", true)];
        let offered = newer_release(&version("0.1.0"), &published).unwrap();
        assert_eq!(offered.version.text(), "0.2.0");
    }

    #[test]
    fn a_marked_program_hears_of_pre_releases_even_if_it_is_not_listed() {
        let published = [release("v1.0.0-rc.2", true), release("v0.9.0", false)];
        let offered = newer_release(&version("1.0.0-rc.1"), &published).unwrap();
        assert_eq!(offered.version.text(), "1.0.0-rc.2");
    }

    #[test]
    fn a_full_release_program_is_not_sent_to_a_pre_release() {
        let published = [
            release("v1.0.0", false),
            release("v1.1.0-rc.1", true),
            release("v0.1.0", true),
        ];
        assert_eq!(newer_release(&version("1.0.0"), &published), None);
        // A full release that is newer is still offered.
        let published = [release("v1.0.0", false), release("v1.1.0", false)];
        let offered = newer_release(&version("1.0.0"), &published).unwrap();
        assert_eq!(offered.version.text(), "1.1.0");
    }

    #[test]
    fn a_pre_release_is_offered_while_no_full_release_exists() {
        // An unlisted build (a copy made from the source) hears of the
        // only releases there are, which are pre-releases.
        let published = [release("v0.3.0", true)];
        let offered = newer_release(&version("0.2.0"), &published).unwrap();
        assert_eq!(offered.version.text(), "0.3.0");
        // Once a full release exists, an unlisted full-version program
        // is sent only to full releases.
        let published = [release("v0.1.0", false), release("v0.3.0", true)];
        assert_eq!(newer_release(&version("0.2.0"), &published), None);
    }

    #[test]
    fn only_a_newer_version_is_installed() {
        assert!(is_installable(&version("0.2.0"), &version("0.3.0")));
        assert!(is_installable(&version("0.9.0"), &version("0.10.0")));
        assert!(!is_installable(&version("0.3.0"), &version("0.3.0")));
        assert!(!is_installable(&version("0.3.0"), &version("0.2.9")));
    }

    #[test]
    fn a_settled_program_is_not_moved_onto_a_marked_version() {
        assert!(!is_installable(&version("1.0.0"), &version("1.1.0-rc.1")));
        // A program with a mark moves on, to the next mark or the release.
        assert!(is_installable(
            &version("1.1.0-rc.1"),
            &version("1.1.0-rc.2")
        ));
        assert!(is_installable(&version("1.1.0-rc.2"), &version("1.1.0")));
    }

    #[test]
    fn the_installer_and_the_notice_order_versions_alike() {
        // Whatever the notice offers to a program without a mark, among
        // full releases, the installer would install.
        let current = version("0.2.0");
        let published = [release("v0.10.0", false), release("v0.9.0", false)];
        let offered = newer_release(&current, &published).unwrap();
        assert!(is_installable(&current, &offered.version));
    }

    #[test]
    fn only_copies_that_own_their_files_update_themselves() {
        for installation in [
            InstallKind::MacApplication,
            InstallKind::WindowsInstaller,
            InstallKind::AppImage,
        ] {
            assert_eq!(installation.updates_itself(), Ok(()));
        }
        assert_eq!(
            InstallKind::SystemPackage.updates_itself(),
            Err(ByHandReason::SystemPackage)
        );
        assert_eq!(
            InstallKind::NotInstalled.updates_itself(),
            Err(ByHandReason::NotInstalled)
        );
        assert!(InstallKind::WindowsInstaller.installs_on_restart());
        assert!(!InstallKind::MacApplication.installs_on_restart());
        assert!(!InstallKind::AppImage.installs_on_restart());
    }

    #[test]
    fn a_program_on_a_pre_release_is_offered_the_full_release_after_it() {
        let published = [release("v0.1.0", true), release("v1.0.0", false)];
        let offered = newer_release(&version("0.1.0"), &published).unwrap();
        assert_eq!(offered.version.text(), "1.0.0");
    }
}
