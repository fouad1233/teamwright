//! The updater of Tauri as the files of an update
//! ([decision 0045](../../../../docs/decisions/0045-installing-updates-from-inside.md)).
//!
//! The address of updates and the public key that every update must be
//! signed with are in `tauri.conf.json`, under `plugins.updater`; they are
//! built into the program and cannot be changed while it runs. The plugin
//! checks the signature of what it downloads against that key and refuses
//! anything else. Nothing here runs unless the manager pressed a button.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tauri::utils::config::BundleType;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Error as UpdaterError, Update, UpdaterExt};
use teamwright_application::{UpdatePackages, UpdateProblem};
use teamwright_domain::{InstallKind, Version, is_installable};

use crate::state::AppState;

/// How long the address of updates may take to answer. The download
/// itself has no limit: it shows its progress, and a slow line is not a
/// failure.
const ANSWER_LIMIT: Duration = Duration::from_secs(30);

/// The files of an update, through the updater plugin of Tauri.
pub struct TauriUpdatePackages {
    app: AppHandle,
    /// What the address of updates offered at the last look.
    found: Mutex<Option<Update>>,
    /// The download, kept until it is put in place. Its signature was
    /// checked when it arrived and is checked again when it is installed.
    downloaded: Mutex<Option<Vec<u8>>>,
}

impl TauriUpdatePackages {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            found: Mutex::new(None),
            downloaded: Mutex::new(None),
        }
    }
}

/// Takes the lock even if a thread panicked while it held it: what is
/// kept is only ever replaced whole.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How this copy was installed, from the mark the bundler of Tauri writes
/// into the program. An AppImage counts only when it runs as one: the
/// AppImage's own runtime names its file, which is what is replaced.
fn install_kind_of(app: &AppHandle) -> InstallKind {
    match tauri::utils::platform::bundle_type() {
        Some(BundleType::Deb | BundleType::Rpm) => InstallKind::SystemPackage,
        Some(BundleType::AppImage) if runs_as_appimage(app) => InstallKind::AppImage,
        Some(BundleType::Msi | BundleType::Nsis) => InstallKind::WindowsInstaller,
        Some(BundleType::App | BundleType::Dmg) => InstallKind::MacApplication,
        _ => InstallKind::NotInstalled,
    }
}

/// Whether the AppImage's runtime started this program and named its file.
#[cfg(target_os = "linux")]
fn runs_as_appimage(app: &AppHandle) -> bool {
    app.env().appimage.is_some()
}

/// Only Linux has AppImages.
#[cfg(not(target_os = "linux"))]
fn runs_as_appimage(_app: &AppHandle) -> bool {
    false
}

/// What a failure of the plugin means to the manager.
fn problem_of(error: UpdaterError) -> UpdateProblem {
    match error {
        UpdaterError::TargetNotFound(_) | UpdaterError::TargetsNotFound(_) => {
            UpdateProblem::NotForThisComputer
        }
        UpdaterError::Minisign(_)
        | UpdaterError::Base64(_)
        | UpdaterError::SignatureUtf8(_)
        | UpdaterError::SignedVersionMismatch { .. }
        | UpdaterError::MissingSignedVersion => UpdateProblem::SignatureRefused,
        UpdaterError::Reqwest(_)
        | UpdaterError::Network(_)
        | UpdaterError::ReleaseNotFound
        | UpdaterError::Serialization(_) => UpdateProblem::NoAnswer,
        other => UpdateProblem::NotInstalled(other.to_string()),
    }
}

impl UpdatePackages for TauriUpdatePackages {
    fn install_kind(&self) -> InstallKind {
        install_kind_of(&self.app)
    }

    fn find(&self) -> Result<Option<Version>, UpdateProblem> {
        let stopping = self.app.clone();
        let updater = self
            .app
            .updater_builder()
            // The rule of the domain decides what is newer, so that the
            // installer and the notice never disagree.
            .version_comparator(|current, offered| {
                match (
                    Version::parse(&current.to_string()),
                    Version::parse(&offered.version.to_string()),
                ) {
                    (Some(current), Some(offered)) => is_installable(&current, &offered),
                    _ => false,
                }
            })
            .timeout(ANSWER_LIMIT)
            // On Windows the installer ends the program at once; the
            // specialists are stopped first, as when it closes.
            .on_before_exit(move || {
                if let Some(state) = stopping.try_state::<AppState>() {
                    let _ = state.work.close();
                }
            })
            .build()
            .map_err(problem_of)?;
        let found = tauri::async_runtime::block_on(updater.check()).map_err(problem_of)?;
        let version = found
            .as_ref()
            .and_then(|update| Version::parse(&update.version));
        *lock(&self.found) = found;
        *lock(&self.downloaded) = None;
        Ok(version)
    }

    fn download(&self, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<(), UpdateProblem> {
        let Some(update) = lock(&self.found).clone() else {
            return Err(UpdateProblem::NoAnswer);
        };
        let bytes = tauri::async_runtime::block_on(update.download(
            |chunk, total| progress(u64::try_from(chunk).unwrap_or(u64::MAX), total),
            || {},
        ))
        .map_err(problem_of)?;
        *lock(&self.downloaded) = Some(bytes);
        Ok(())
    }

    fn install(&self) -> Result<(), UpdateProblem> {
        let update = lock(&self.found).clone();
        let bytes = lock(&self.downloaded).take();
        let (Some(update), Some(bytes)) = (update, bytes) else {
            return Err(UpdateProblem::NotInstalled(
                "nothing was downloaded".to_owned(),
            ));
        };
        update.install(&bytes).map_err(problem_of)
    }

    fn restart(&self) {
        // Goes through the end of the program, so that the specialists are
        // stopped as when the manager closes it.
        self.app.request_restart();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_of_the_plugin_reaches_the_manager_as_what_it_means() {
        assert_eq!(
            problem_of(UpdaterError::TargetsNotFound(vec![
                "linux-x86_64".to_owned()
            ])),
            UpdateProblem::NotForThisComputer
        );
        assert_eq!(
            problem_of(UpdaterError::SignedVersionMismatch {
                signed: "0.2.0".to_owned(),
                announced: "0.3.0".to_owned(),
            }),
            UpdateProblem::SignatureRefused
        );
        assert_eq!(
            problem_of(UpdaterError::MissingSignedVersion),
            UpdateProblem::SignatureRefused
        );
        assert_eq!(
            problem_of(UpdaterError::ReleaseNotFound),
            UpdateProblem::NoAnswer
        );
        assert_eq!(
            problem_of(UpdaterError::Network("status 404".to_owned())),
            UpdateProblem::NoAnswer
        );
        assert!(matches!(
            problem_of(UpdaterError::InvalidUpdaterFormat),
            UpdateProblem::NotInstalled(_)
        ));
    }
}
