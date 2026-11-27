//! Installing a newer version from inside the program
//! ([decision 0045](../../../docs/decisions/0045-installing-updates-from-inside.md)).
//!
//! The notice of decision 0043 says that a newer version is out. When the
//! manager presses the button to install it, the program asks the address
//! of updates what it offers for this computer, downloads it with a
//! progress the manager can see, has its signature checked against the key
//! built into the program, and puts it in place. It never does any of this
//! by itself, and it restarts only when the manager says so, and not while
//! specialists work unless the manager says yes to stopping them.
//!
//! What the address of updates offers decides what is installed: it is
//! what can actually be installed. The notice only says that something
//! newer is out. Both order versions by the rule of the domain.

use std::sync::{Arc, Mutex, MutexGuard};

use teamwright_domain::{ByHandReason, InstallKind, Version, is_installable};

use crate::error::ApplicationError;

/// Why a step of an update did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateProblem {
    /// The address of updates could not be reached, or did not answer with
    /// what an update needs.
    NoAnswer,
    /// The address of updates has no update for this kind of computer.
    NotForThisComputer,
    /// The download does not carry a signature made with the key built
    /// into this program. It was thrown away.
    SignatureRefused,
    /// The new version could not be put in place, for the reason given.
    NotInstalled(String),
}

/// The files of an update and what can be done with them. The desktop
/// shell implements it with the updater of Tauri.
pub trait UpdatePackages: Send + Sync {
    /// How this copy of the program was installed.
    fn install_kind(&self) -> InstallKind;

    /// Asks the address of updates what it offers for this computer. `None`
    /// means it offers nothing that may be installed over this program.
    ///
    /// # Errors
    ///
    /// Why there is no answer.
    fn find(&self) -> Result<Option<Version>, UpdateProblem>;

    /// Downloads what [`Self::find`] found and checks its signature. The
    /// progress is told as bytes received so far and the whole size, when
    /// the address says it.
    ///
    /// # Errors
    ///
    /// Why the download failed or was refused.
    fn download(&self, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<(), UpdateProblem>;

    /// Puts the downloaded version in place. On Windows this starts the
    /// installer and ends the program.
    ///
    /// # Errors
    ///
    /// Why it could not be put in place.
    fn install(&self) -> Result<(), UpdateProblem>;

    /// Ends the program and starts it again, which starts the new version.
    fn restart(&self);
}

/// Where an install stands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InstallPhase {
    /// Nothing was asked.
    #[default]
    Idle,
    /// The address of updates is being asked.
    Looking,
    /// A version is being downloaded.
    Downloading {
        /// The version, as it reads.
        version: String,
        /// How many bytes have arrived.
        received: u64,
        /// How many bytes there are, when the address said it.
        total: Option<u64>,
    },
    /// A version was downloaded, its signature checked, and, except on
    /// Windows, put in place. A restart starts it.
    Ready {
        /// The version, as it reads.
        version: String,
    },
    /// The last try did not succeed.
    Failed(InstallFailure),
}

/// Why the last try to install did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallFailure {
    /// The address of updates offers nothing newer for this computer yet,
    /// although the notice may know of a newer release.
    NothingNewer,
    /// One of the steps did not succeed.
    Problem(UpdateProblem),
    /// This copy cannot install its own updates.
    ByHand(ByHandReason),
}

/// What the interface shows about installing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallState {
    /// Whether this copy installs its own updates, or why not.
    pub way: Result<(), ByHandReason>,
    /// Whether the new version is put in place only at the restart.
    pub installs_on_restart: bool,
    /// Where the install stands.
    pub phase: InstallPhase,
}

/// The use cases of installing an update.
#[derive(Clone)]
pub struct InstallService {
    current: Option<Version>,
    packages: Arc<dyn UpdatePackages>,
    phase: Arc<Mutex<InstallPhase>>,
}

impl InstallService {
    /// Creates the service for a program of the given version.
    #[must_use]
    pub fn new(this_version: &str, packages: Arc<dyn UpdatePackages>) -> Self {
        Self {
            current: Version::parse(this_version),
            packages,
            phase: Arc::new(Mutex::new(InstallPhase::Idle)),
        }
    }

    fn phase(&self) -> MutexGuard<'_, InstallPhase> {
        // A panic while the phase was held leaves a phase that is still
        // whole: it is only ever replaced.
        self.phase
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn set(&self, phase: InstallPhase) {
        *self.phase() = phase;
    }

    /// Where the install stands, without doing anything.
    #[must_use]
    pub fn state(&self) -> InstallState {
        let kind = self.packages.install_kind();
        InstallState {
            way: kind.updates_itself(),
            installs_on_restart: kind.installs_on_restart(),
            phase: self.phase().clone(),
        }
    }

    /// Whether an install is under way, so that a second press does not
    /// start another.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        matches!(
            *self.phase(),
            InstallPhase::Looking | InstallPhase::Downloading { .. }
        )
    }

    /// Asks the address of updates, downloads what it offers, has the
    /// signature checked and puts the new version in place, because the
    /// manager pressed the button: [`Self::begin`], then [`Self::carry_on`].
    pub fn install(&self) -> InstallState {
        if self.begin() {
            self.carry_on()
        } else {
            self.state()
        }
    }

    /// Marks that the manager pressed the button, and says whether an
    /// install should now be carried on. A press while an install is under
    /// way, or after one is ready, starts nothing more.
    #[must_use]
    pub fn begin(&self) -> bool {
        let mut phase = self.phase();
        if matches!(
            *phase,
            InstallPhase::Looking | InstallPhase::Downloading { .. } | InstallPhase::Ready { .. }
        ) {
            return false;
        }
        *phase = InstallPhase::Looking;
        true
    }

    /// Carries on an install that [`Self::begin`] started. It waits for the
    /// network, so the caller runs it on a thread of its own and reads
    /// [`Self::state`] meanwhile. A failure is not an error: the state says
    /// what went wrong.
    pub fn carry_on(&self) -> InstallState {
        let kind = self.packages.install_kind();
        let outcome = match kind.updates_itself() {
            Err(reason) => Err(InstallFailure::ByHand(reason)),
            Ok(()) => self.find_download_and_place(kind),
        };
        self.set(match outcome {
            Ok(version) => InstallPhase::Ready { version },
            Err(failure) => InstallPhase::Failed(failure),
        });
        self.state()
    }

    fn find_download_and_place(&self, kind: InstallKind) -> Result<String, InstallFailure> {
        let offered = self
            .packages
            .find()
            .map_err(InstallFailure::Problem)?
            .ok_or(InstallFailure::NothingNewer)?;
        // The shell asks the same rule of the address's answer; asking it
        // again here keeps the rule in one place for every implementation.
        let is_newer = self
            .current
            .as_ref()
            .is_some_and(|current| is_installable(current, &offered));
        if !is_newer {
            return Err(InstallFailure::NothingNewer);
        }
        let version = offered.text();
        self.set(InstallPhase::Downloading {
            version: version.clone(),
            received: 0,
            total: None,
        });
        let mut received = 0_u64;
        self.packages
            .download(&mut |chunk, total| {
                received = received.saturating_add(chunk);
                self.set(InstallPhase::Downloading {
                    version: version.clone(),
                    received,
                    total,
                });
            })
            .map_err(InstallFailure::Problem)?;
        if !kind.installs_on_restart() {
            self.packages.install().map_err(InstallFailure::Problem)?;
        }
        Ok(version)
    }

    /// Restarts the program into the new version, because the manager
    /// pressed "Restart now". While specialists work on this computer it
    /// refuses, unless the manager said yes to stopping them: a restart
    /// ends their turns.
    ///
    /// # Errors
    ///
    /// - [`ApplicationError::UpdateNotReady`] if no update is ready.
    /// - [`ApplicationError::SpecialistsAtWork`] if specialists work and
    ///   the manager has not said yes.
    ///
    /// When the new version could not be put in place (on Windows, where
    /// that happens now), the state says why and the program goes on.
    pub fn restart(
        &self,
        turns_running: usize,
        even_while_working: bool,
    ) -> Result<InstallState, ApplicationError> {
        if !matches!(*self.phase(), InstallPhase::Ready { .. }) {
            return Err(ApplicationError::UpdateNotReady);
        }
        if turns_running > 0 && !even_while_working {
            return Err(ApplicationError::SpecialistsAtWork {
                count: turns_running,
            });
        }
        if self.packages.install_kind().installs_on_restart() {
            // On Windows the installer ends the program; a return means it
            // could not start.
            if let Err(problem) = self.packages.install() {
                self.set(InstallPhase::Failed(InstallFailure::Problem(problem)));
                return Ok(self.state());
            }
        }
        self.packages.restart();
        Ok(self.state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Files of an update that answer as told, and remember what was done.
    struct Packages {
        install_kind: InstallKind,
        found: Result<Option<Version>, UpdateProblem>,
        download: Result<(), UpdateProblem>,
        install: Result<(), UpdateProblem>,
        done: Mutex<Vec<&'static str>>,
    }

    impl Packages {
        fn offering(version: &str) -> Self {
            Self {
                install_kind: InstallKind::AppImage,
                found: Ok(Version::parse(version)),
                download: Ok(()),
                install: Ok(()),
                done: Mutex::new(Vec::new()),
            }
        }

        fn done(&self) -> Vec<&'static str> {
            self.done.lock().unwrap().clone()
        }
    }

    impl UpdatePackages for Packages {
        fn install_kind(&self) -> InstallKind {
            self.install_kind
        }

        fn find(&self) -> Result<Option<Version>, UpdateProblem> {
            self.done.lock().unwrap().push("find");
            self.found.clone()
        }

        fn download(
            &self,
            progress: &mut dyn FnMut(u64, Option<u64>),
        ) -> Result<(), UpdateProblem> {
            self.done.lock().unwrap().push("download");
            progress(40, Some(100));
            progress(60, Some(100));
            self.download.clone()
        }

        fn install(&self) -> Result<(), UpdateProblem> {
            self.done.lock().unwrap().push("install");
            self.install.clone()
        }

        fn restart(&self) {
            self.done.lock().unwrap().push("restart");
        }
    }

    fn service(packages: Packages) -> (InstallService, Arc<Packages>) {
        let packages = Arc::new(packages);
        (InstallService::new("0.2.0", packages.clone()), packages)
    }

    #[test]
    fn nothing_happens_until_the_manager_presses_the_button() {
        let (service, packages) = service(Packages::offering("0.3.0"));
        let state = service.state();
        assert_eq!(state.phase, InstallPhase::Idle);
        assert_eq!(state.way, Ok(()));
        assert!(packages.done().is_empty());
    }

    #[test]
    fn a_press_finds_downloads_and_puts_the_version_in_place_but_does_not_restart() {
        let (service, packages) = service(Packages::offering("0.3.0"));
        let state = service.install();
        assert_eq!(
            state.phase,
            InstallPhase::Ready {
                version: "0.3.0".to_owned()
            }
        );
        assert_eq!(packages.done(), ["find", "download", "install"]);
    }

    #[test]
    fn on_windows_the_version_is_put_in_place_only_at_the_restart() {
        let mut packages = Packages::offering("0.3.0");
        packages.install_kind = InstallKind::WindowsInstaller;
        let (service, packages) = service(packages);
        let state = service.install();
        assert!(state.installs_on_restart);
        assert_eq!(packages.done(), ["find", "download"]);
        service.restart(0, false).unwrap();
        assert_eq!(packages.done(), ["find", "download", "install", "restart"]);
    }

    #[test]
    fn a_signature_that_does_not_match_is_refused_and_nothing_is_installed() {
        let mut packages = Packages::offering("0.3.0");
        packages.download = Err(UpdateProblem::SignatureRefused);
        let (service, packages) = service(packages);
        let state = service.install();
        assert_eq!(
            state.phase,
            InstallPhase::Failed(InstallFailure::Problem(UpdateProblem::SignatureRefused))
        );
        assert_eq!(packages.done(), ["find", "download"]);
        assert!(matches!(
            service.restart(0, true),
            Err(ApplicationError::UpdateNotReady)
        ));
    }

    #[test]
    fn an_address_with_nothing_newer_says_so() {
        let mut packages = Packages::offering("0.3.0");
        packages.found = Ok(None);
        let (service, _) = service(packages);
        assert_eq!(
            service.install().phase,
            InstallPhase::Failed(InstallFailure::NothingNewer)
        );
    }

    #[test]
    fn an_offer_the_rule_of_the_domain_does_not_allow_is_not_downloaded() {
        // An address that answers with the same version, or with a release
        // candidate for a settled program, is not followed.
        for offered in ["0.2.0", "0.1.0", "0.3.0-rc.1"] {
            let (service, packages) = service(Packages::offering(offered));
            assert_eq!(
                service.install().phase,
                InstallPhase::Failed(InstallFailure::NothingNewer),
                "{offered}"
            );
            assert_eq!(packages.done(), ["find"]);
        }
    }

    #[test]
    fn a_failure_can_be_tried_again() {
        let mut packages = Packages::offering("0.3.0");
        packages.found = Err(UpdateProblem::NoAnswer);
        let (service, packages) = service(packages);
        service.install();
        service.install();
        assert_eq!(packages.done(), ["find", "find"]);
    }

    #[test]
    fn a_second_press_after_a_ready_update_does_nothing_more() {
        let (service, packages) = service(Packages::offering("0.3.0"));
        service.install();
        service.install();
        assert_eq!(packages.done(), ["find", "download", "install"]);
    }

    /// Files of an update whose download reads the state of the service
    /// while it reports progress, as the window does while it waits.
    struct Watched {
        reader: Mutex<Option<InstallService>>,
        seen: Mutex<Vec<InstallPhase>>,
    }

    impl UpdatePackages for Watched {
        fn install_kind(&self) -> InstallKind {
            InstallKind::MacApplication
        }

        fn find(&self) -> Result<Option<Version>, UpdateProblem> {
            Ok(Version::parse("0.3.0"))
        }

        fn download(
            &self,
            progress: &mut dyn FnMut(u64, Option<u64>),
        ) -> Result<(), UpdateProblem> {
            let reader = self.reader.lock().unwrap().clone().unwrap();
            for chunk in [30, 70] {
                progress(chunk, Some(100));
                self.seen.lock().unwrap().push(reader.state().phase);
            }
            assert!(reader.is_busy());
            Ok(())
        }

        fn install(&self) -> Result<(), UpdateProblem> {
            Ok(())
        }

        fn restart(&self) {}
    }

    #[test]
    fn the_progress_of_the_download_can_be_read_while_it_runs() {
        let packages = Arc::new(Watched {
            reader: Mutex::new(None),
            seen: Mutex::new(Vec::new()),
        });
        let service = InstallService::new("0.2.0", packages.clone());
        *packages.reader.lock().unwrap() = Some(service.clone());
        service.install();
        let downloading = |received| InstallPhase::Downloading {
            version: "0.3.0".to_owned(),
            received,
            total: Some(100),
        };
        assert_eq!(
            *packages.seen.lock().unwrap(),
            [downloading(30), downloading(100)]
        );
        assert!(!service.is_busy());
    }

    #[test]
    fn a_copy_of_the_system_package_is_updated_by_hand() {
        let mut packages = Packages::offering("0.3.0");
        packages.install_kind = InstallKind::SystemPackage;
        let (service, packages) = service(packages);
        let state = service.install();
        assert_eq!(state.way, Err(ByHandReason::SystemPackage));
        assert_eq!(
            state.phase,
            InstallPhase::Failed(InstallFailure::ByHand(ByHandReason::SystemPackage))
        );
        assert!(packages.done().is_empty());
    }

    #[test]
    fn a_restart_waits_for_the_managers_yes_while_specialists_work() {
        let (service, packages) = service(Packages::offering("0.3.0"));
        service.install();
        let refused = service.restart(2, false).unwrap_err();
        assert!(matches!(
            refused,
            ApplicationError::SpecialistsAtWork { count: 2 }
        ));
        assert_eq!(
            refused.to_string(),
            "2 specialists are at work; restarting now would stop them"
        );
        assert!(!packages.done().contains(&"restart"));
        service.restart(2, true).unwrap();
        assert!(packages.done().contains(&"restart"));
    }

    #[test]
    fn a_windows_installer_that_cannot_start_is_said_and_the_program_goes_on() {
        let mut packages = Packages::offering("0.3.0");
        packages.install_kind = InstallKind::WindowsInstaller;
        packages.install = Err(UpdateProblem::NotInstalled("denied".to_owned()));
        let (service, packages) = service(packages);
        service.install();
        let state = service.restart(0, false).unwrap();
        assert_eq!(
            state.phase,
            InstallPhase::Failed(InstallFailure::Problem(UpdateProblem::NotInstalled(
                "denied".to_owned()
            )))
        );
        assert!(!packages.done().contains(&"restart"));
    }
}
