//! Reading the answer of `claude auth status --json`.
//!
//! The answer also holds the user's e-mail address and organization. They
//! are not read into any field here, so they cannot end up in Teamwright's
//! data, logs or error messages.

use serde::Deserialize;
use teamwright_application::{AgentRuntimeError, SignIn};

/// The part of the answer Teamwright uses. Every other field is ignored.
#[derive(Deserialize)]
struct AuthStatus {
    #[serde(rename = "loggedIn")]
    logged_in: bool,
    #[serde(rename = "authMethod", default)]
    method: Option<String>,
    #[serde(rename = "apiProvider", default)]
    provider: Option<String>,
    #[serde(rename = "subscriptionType", default)]
    subscription: Option<String>,
}

/// Reads the sign-in state from the program's answer.
///
/// Claude Code exits with 0 when the user is signed in and with 1 when not;
/// in both cases it writes the JSON answer. The answer, not the exit code,
/// decides.
///
/// # Errors
///
/// Returns [`AgentRuntimeError::UnexpectedAnswer`] if the answer is not the
/// expected JSON. The message never repeats the answer.
pub fn parse_sign_in(answer: &str) -> Result<SignIn, AgentRuntimeError> {
    let status: AuthStatus =
        serde_json::from_str(answer.trim()).map_err(|_| AgentRuntimeError::UnexpectedAnswer {
            message: "the sign-in status was not the expected JSON".to_owned(),
        })?;
    if !status.logged_in {
        return Ok(SignIn::SignedOut);
    }
    Ok(SignIn::SignedIn {
        method: status.method.unwrap_or_default(),
        provider: status.provider.unwrap_or_default(),
        subscription: status.subscription.filter(|kind| !kind.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The shape of the answers of Claude Code 2.1.322, with sample values.
    const SIGNED_IN: &str = r#"{
  "loggedIn": true,
  "authMethod": "claude.ai",
  "apiProvider": "firstParty",
  "analyticsDisabled": false,
  "projectsDirectory": "C:\\Users\\sample\\.claude\\projects",
  "configDirectory": "C:\\Users\\sample\\.claude",
  "loginExpiresAt": "2026-12-01T00:00:00.000Z",
  "email": "sample@example.com",
  "orgId": "00000000-0000-0000-0000-000000000000",
  "orgName": "Sample Org",
  "subscriptionType": "max",
  "storedClaudeAiLogin": true,
  "planTier": "max20"
}"#;

    const SIGNED_OUT: &str = r#"{
  "loggedIn": false,
  "authMethod": "none",
  "apiProvider": "firstParty",
  "analyticsDisabled": false,
  "projectsDirectory": "C:\\Users\\sample\\.claude\\projects",
  "configDirectory": "C:/Users/sample/.claude",
  "loginExpiresAt": null
}"#;

    #[test]
    fn a_signed_in_user_is_recognised_with_method_and_plan() {
        assert_eq!(
            parse_sign_in(SIGNED_IN).unwrap(),
            SignIn::SignedIn {
                method: "claude.ai".to_owned(),
                provider: "firstParty".to_owned(),
                subscription: Some("max".to_owned()),
            }
        );
    }

    #[test]
    fn a_signed_out_user_is_recognised() {
        assert_eq!(parse_sign_in(SIGNED_OUT).unwrap(), SignIn::SignedOut);
    }

    #[test]
    fn personal_details_are_not_kept() {
        let kept = format!("{:?}", parse_sign_in(SIGNED_IN).unwrap());
        assert!(!kept.contains("sample@example.com"));
        assert!(!kept.contains("Sample Org"));
    }

    #[test]
    fn an_answer_that_is_not_json_is_an_error_that_does_not_repeat_it() {
        let error = parse_sign_in("secret text").unwrap_err();
        assert_eq!(error.code(), "agent_unexpected_answer");
        assert!(!error.to_string().contains("secret"));
        assert!(parse_sign_in(r#"{"authMethod":"claude.ai"}"#).is_err());
    }
}
