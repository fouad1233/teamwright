//! Earlier conversations and their digests on the wire
//! ([decision 0041](../../../../../docs/decisions/0041-earlier-conversations-as-digests.md)).
//! `src/shared/api/types.ts` mirrors these types.

use serde::{Deserialize, Serialize};
use teamwright_application::{ConversationsOfFolder, DigestToKeep, EarlierConversationCard};
use teamwright_domain::{Digest, DigestDraft};

use super::{moment_to_text, parse_moment};
use crate::error::CommandError;

/// An earlier conversation as the list shows it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EarlierConversationDto {
    source: String,
    held_in: String,
    title: Option<String>,
    first_request: Option<String>,
    started_at: Option<String>,
    ended_at: Option<String>,
    size_bytes: u64,
    is_kept: bool,
}

impl From<EarlierConversationCard> for EarlierConversationDto {
    fn from(card: EarlierConversationCard) -> Self {
        Self {
            source: card.source,
            held_in: card.held_in,
            title: card.title,
            first_request: card.first_request,
            started_at: card.started_at.map(moment_to_text),
            ended_at: card.ended_at.map(moment_to_text),
            size_bytes: card.size_bytes,
            is_kept: card.is_kept,
        }
    }
}

/// The earlier conversations held in one folder.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationsOfFolderDto {
    folder: String,
    is_of_project: bool,
    conversations: Vec<EarlierConversationDto>,
}

impl From<ConversationsOfFolder> for ConversationsOfFolderDto {
    fn from(folder: ConversationsOfFolder) -> Self {
        Self {
            folder: folder.folder,
            is_of_project: folder.is_of_project,
            conversations: folder
                .conversations
                .into_iter()
                .map(EarlierConversationDto::from)
                .collect(),
        }
    }
}

/// A digest made by the rules, for the manager to read and correct. The
/// interface sends it back, corrected, to keep it.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DigestDraftDto {
    source: String,
    held_in: String,
    title: String,
    text: String,
    started_at: Option<String>,
    ended_at: Option<String>,
}

impl From<DigestDraft> for DigestDraftDto {
    fn from(draft: DigestDraft) -> Self {
        Self {
            source: draft.source,
            held_in: draft.held_in,
            title: draft.title.to_string(),
            text: draft.text,
            started_at: draft.started_at.map(moment_to_text),
            ended_at: draft.ended_at.map(moment_to_text),
        }
    }
}

impl TryFrom<DigestDraftDto> for DigestToKeep {
    type Error = CommandError;

    fn try_from(draft: DigestDraftDto) -> Result<Self, Self::Error> {
        Ok(Self {
            source: draft.source,
            held_in: draft.held_in,
            title: draft.title,
            text: draft.text,
            started_at: draft.started_at.as_deref().map(parse_moment).transpose()?,
            ended_at: draft.ended_at.as_deref().map(parse_moment).transpose()?,
        })
    }
}

/// A digest kept with a project.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DigestDto {
    id: String,
    project_id: String,
    source: String,
    held_in: String,
    title: String,
    text: String,
    started_at: Option<String>,
    ended_at: Option<String>,
    kept_at: String,
}

impl From<&Digest> for DigestDto {
    fn from(digest: &Digest) -> Self {
        Self {
            id: digest.id().to_string(),
            project_id: digest.project_id().to_string(),
            source: digest.source().source().to_owned(),
            held_in: digest.source().held_in().to_owned(),
            title: digest.title().to_string(),
            text: digest.text().as_str().to_owned(),
            started_at: digest.started_at().map(moment_to_text),
            ended_at: digest.ended_at().map(moment_to_text),
            kept_at: moment_to_text(digest.kept_at()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_comes_back_as_what_is_kept() {
        let draft: DigestDraftDto = serde_json::from_value(serde_json::json!({
            "source": "-home-ada-air/abc",
            "heldIn": "/home/ada/air",
            "title": "Sensor plan",
            "text": "First request:\nPlan it.",
            "startedAt": "2026-10-03T09:00:00.000Z",
            "endedAt": null
        }))
        .unwrap();

        let kept = DigestToKeep::try_from(draft).unwrap();

        assert_eq!(kept.source, "-home-ada-air/abc");
        assert_eq!(kept.held_in, "/home/ada/air");
        assert_eq!(kept.text, "First request:\nPlan it.");
        assert_eq!(
            kept.started_at.map(moment_to_text).as_deref(),
            Some("2026-10-03T09:00:00.000Z")
        );
        assert_eq!(kept.ended_at, None);
    }

    #[test]
    fn a_draft_with_a_moment_that_is_not_one_is_refused() {
        let draft: DigestDraftDto = serde_json::from_value(serde_json::json!({
            "source": "a/b", "heldIn": "", "title": "T", "text": "x",
            "startedAt": "yesterday", "endedAt": null
        }))
        .unwrap();

        assert!(DigestToKeep::try_from(draft).is_err());
    }
}
