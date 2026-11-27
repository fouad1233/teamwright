//! Installing an update from inside the program, on the wire
//! ([decision 0045](../../../../../docs/decisions/0045-installing-updates-from-inside.md)).
//! `src/shared/api/types.ts` mirrors these types (`UpdateInstall`).

use serde::Serialize;
use teamwright_application::{InstallFailure, InstallPhase, InstallState, UpdateProblem};
use teamwright_domain::ByHandReason;

/// Where installing an update stands, and what the part "Version" needs to
/// say about it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstallDto {
    /// Whether this copy installs its own updates.
    pub updates_itself: bool,
    /// Why not, when it does not: `systemPackage` or `notInstalled`.
    pub by_hand_reason: Option<&'static str>,
    /// Whether the new version is put in place only at the restart, as on
    /// Windows.
    pub installs_on_restart: bool,
    /// Where updates come from, as the manager reads it.
    pub source: String,
    /// How many specialists work on this computer now. A restart would
    /// stop them.
    pub specialists_at_work: usize,
    /// Where the install stands.
    pub progress: InstallProgressDto,
}

/// Where an install stands, told by its `phase`.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum InstallProgressDto {
    /// Nothing was asked.
    Idle,
    /// The address of updates is asked.
    Looking,
    /// A version is downloading.
    Downloading {
        version: String,
        received: u64,
        total: Option<u64>,
    },
    /// Downloaded, its signature checked, and waiting for a restart.
    Ready { version: String },
    /// The last try did not succeed, for the reason `failure` names.
    Failed {
        failure: &'static str,
        detail: Option<String>,
    },
}

/// The name of a reason to install by hand on the wire.
const fn by_hand_name(reason: ByHandReason) -> &'static str {
    match reason {
        ByHandReason::SystemPackage => "systemPackage",
        ByHandReason::NotInstalled => "notInstalled",
    }
}

impl From<InstallPhase> for InstallProgressDto {
    fn from(phase: InstallPhase) -> Self {
        match phase {
            InstallPhase::Idle => Self::Idle,
            InstallPhase::Looking => Self::Looking,
            InstallPhase::Downloading {
                version,
                received,
                total,
            } => Self::Downloading {
                version,
                received,
                total,
            },
            InstallPhase::Ready { version } => Self::Ready { version },
            InstallPhase::Failed(failure) => {
                let (failure, detail) = match failure {
                    InstallFailure::NothingNewer => ("nothingNewer", None),
                    InstallFailure::ByHand(reason) => {
                        ("byHand", Some(by_hand_name(reason).to_owned()))
                    }
                    InstallFailure::Problem(UpdateProblem::NoAnswer) => ("noAnswer", None),
                    InstallFailure::Problem(UpdateProblem::NotForThisComputer) => {
                        ("notForThisComputer", None)
                    }
                    InstallFailure::Problem(UpdateProblem::SignatureRefused) => {
                        ("signatureRefused", None)
                    }
                    InstallFailure::Problem(UpdateProblem::NotInstalled(why)) => {
                        ("notInstalled", Some(why))
                    }
                };
                Self::Failed { failure, detail }
            }
        }
    }
}

impl UpdateInstallDto {
    /// The state of installing, with where updates come from and how many
    /// specialists work.
    pub fn new(state: InstallState, source: String, specialists_at_work: usize) -> Self {
        Self {
            updates_itself: state.way.is_ok(),
            by_hand_reason: state.way.err().map(by_hand_name),
            installs_on_restart: state.installs_on_restart,
            source,
            specialists_at_work,
            progress: state.phase.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_of_installing_goes_out_in_camel_case_with_its_phase() {
        let state = InstallState {
            way: Ok(()),
            installs_on_restart: false,
            phase: InstallPhase::Downloading {
                version: "0.3.0".to_owned(),
                received: 512,
                total: Some(2048),
            },
        };
        assert_eq!(
            serde_json::to_value(UpdateInstallDto::new(state, "github.com/o/r".to_owned(), 1))
                .unwrap(),
            serde_json::json!({
                "updatesItself": true,
                "byHandReason": null,
                "installsOnRestart": false,
                "source": "github.com/o/r",
                "specialistsAtWork": 1,
                "progress": {
                    "phase": "downloading",
                    "version": "0.3.0",
                    "received": 512,
                    "total": 2048
                }
            })
        );
    }

    #[test]
    fn a_refused_signature_and_a_copy_updated_by_hand_have_their_names() {
        let refused: InstallProgressDto =
            InstallPhase::Failed(InstallFailure::Problem(UpdateProblem::SignatureRefused)).into();
        assert_eq!(
            serde_json::to_value(refused).unwrap(),
            serde_json::json!({ "phase": "failed", "failure": "signatureRefused", "detail": null })
        );
        let state = InstallState {
            way: Err(ByHandReason::SystemPackage),
            installs_on_restart: false,
            phase: InstallPhase::Idle,
        };
        let value = serde_json::to_value(UpdateInstallDto::new(state, String::new(), 0)).unwrap();
        assert_eq!(value["updatesItself"], false);
        assert_eq!(value["byHandReason"], "systemPackage");
        assert_eq!(value["progress"], serde_json::json!({ "phase": "idle" }));
    }
}
