//! Earlier conversations brought into a project as digests
//! ([decision 0041](../../../docs/decisions/0041-earlier-conversations-as-digests.md)).
//!
//! The manager often talked a project through with Claude Code before it
//! came to Teamwright. These use cases find those conversations on this
//! computer, make a digest of one by the rules of the domain
//! ([`DigestDraft::of`]), and keep the digest the manager read and
//! corrected with the project. The Advisor starts from the kept digests
//! (see [`crate::work`]). The conversation itself stays where it is: it is
//! read, never copied, changed or synced.

use std::cmp::Reverse;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use teamwright_domain::digest::cut;
use teamwright_domain::{
    Digest, DigestDraft, DigestId, DigestSource, DigestText, DigestTitle, ProjectId, Timestamp,
    hide_secrets,
};

use crate::error::ApplicationError;
use crate::places::PlacesService;
use crate::ports::{Clock, DigestRepository, EarlierConversations, FoundConversation};

/// How many characters of the first request a list of conversations shows.
const FIRST_REQUEST_IN_LIST: usize = 240;

/// One earlier conversation as the manager sees it in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarlierConversationCard {
    /// How it is found again, to make its digest.
    pub source: String,
    /// The folder it was held in.
    pub held_in: String,
    /// Its title, if it has one, with what looks like a secret hidden.
    pub title: Option<String>,
    /// The beginning of its first request on one line, with what looks
    /// like a secret hidden.
    pub first_request: Option<String>,
    /// When it began, if known.
    pub started_at: Option<Timestamp>,
    /// When it last changed, if known.
    pub ended_at: Option<Timestamp>,
    /// The size of its file, in bytes: how long it is.
    pub size_bytes: u64,
    /// Whether a digest of it is kept with the project already.
    pub is_kept: bool,
}

/// The earlier conversations held in one folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationsOfFolder {
    /// The folder.
    pub folder: String,
    /// Whether it is a folder of the project, or lies inside one.
    pub is_of_project: bool,
    /// Its conversations, the newest first.
    pub conversations: Vec<EarlierConversationCard>,
}

/// A digest as the manager keeps it, after reading and correcting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestToKeep {
    /// How the conversation is found again.
    pub source: String,
    /// The folder it was held in.
    pub held_in: String,
    /// The title, as the manager left it.
    pub title: String,
    /// The account, as the manager left it.
    pub text: String,
    /// When the conversation began, if known.
    pub started_at: Option<Timestamp>,
    /// When it last changed, if known.
    pub ended_at: Option<Timestamp>,
}

/// The use cases of earlier conversations and their digests.
#[derive(Clone)]
pub struct DigestService {
    places: PlacesService,
    digests: Arc<dyn DigestRepository>,
    reader: Arc<dyn EarlierConversations>,
    clock: Arc<dyn Clock>,
}

impl DigestService {
    /// Creates the service.
    #[must_use]
    pub fn new(
        places: PlacesService,
        digests: Arc<dyn DigestRepository>,
        reader: Arc<dyn EarlierConversations>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            places,
            digests,
            reader,
            clock,
        }
    }

    /// The earlier conversations on this computer, grouped by the folder
    /// they were held in. The folders of the project come first, because a
    /// conversation held there is likely about it; then the other folders,
    /// the one with the newest conversation first. Inside a folder the
    /// newest conversation comes first. A conversation whose digest is kept
    /// with the project already says so.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, or if the place the
    /// conversations are kept in cannot be read.
    pub fn find_earlier(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<ConversationsOfFolder>, ApplicationError> {
        let places = self.places.places(project_id)?;
        let project_folders: Vec<PathBuf> = std::iter::once(places.work_folder)
            .chain(places.reference_folders)
            .collect();
        let kept: HashSet<String> = self
            .digests
            .list_for_project(project_id)?
            .iter()
            .map(|digest| digest.source().source().to_owned())
            .collect();
        let found = self.reader.list()?;
        Ok(group_by_folder(found, &project_folders, &kept))
    }

    /// Makes the digest of an earlier conversation by the rules, for the
    /// manager to read and correct. Nothing is kept.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the source names
    /// no conversation (`not_found`), if the conversation holds no request
    /// of the manager (`rule_broken`), or if it cannot be read.
    pub fn draft(
        &self,
        project_id: ProjectId,
        source: &str,
    ) -> Result<DigestDraft, ApplicationError> {
        self.places.places(project_id)?;
        let conversation = self
            .reader
            .read(source)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "earlier conversation",
                id: source.to_owned(),
            })?;
        Ok(DigestDraft::of(&conversation)?)
    }

    /// Keeps a digest with a project, as the manager read and corrected
    /// it. It is stored and logged like the rest of the project, so it
    /// reaches the other computers.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the title or the
    /// text breaks a rule (`rule_broken`), if a digest of the same
    /// conversation is kept with the project already
    /// (`digest_already_kept`), or if storage fails.
    pub fn keep(
        &self,
        project_id: ProjectId,
        kept: DigestToKeep,
    ) -> Result<Digest, ApplicationError> {
        self.places.places(project_id)?;
        let source = DigestSource::new(&kept.source, &kept.held_in)?;
        let title = DigestTitle::parse(&kept.title)?;
        let text = DigestText::parse(&kept.text)?;
        if let Some(earlier) = self
            .digests
            .list_for_project(project_id)?
            .into_iter()
            .find(|digest| digest.source().source() == source.source())
        {
            return Err(ApplicationError::DigestAlreadyKept {
                title: earlier.title().to_string(),
            });
        }
        let digest = Digest::keep(
            project_id,
            source,
            title,
            text,
            (kept.started_at, kept.ended_at),
            self.clock.now(),
        );
        self.digests.save(&digest)?;
        Ok(digest)
    }

    /// The digests kept with a project, the one kept last first.
    ///
    /// # Errors
    ///
    /// Returns an error if storage cannot be read.
    pub fn digests_of(&self, project_id: ProjectId) -> Result<Vec<Digest>, ApplicationError> {
        let mut digests = self.digests.list_for_project(project_id)?;
        newest_kept_first(&mut digests);
        Ok(digests)
    }

    /// Removes a digest from its project. The conversation it was made
    /// from is not touched.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such digest, or if storage fails.
    pub fn remove(&self, digest_id: DigestId) -> Result<(), ApplicationError> {
        if self.digests.find(digest_id)?.is_none() {
            return Err(ApplicationError::NotFound {
                kind: "digest",
                id: digest_id.to_string(),
            });
        }
        self.digests.delete(digest_id)?;
        Ok(())
    }
}

/// Orders digests with the one kept last first; two kept at one moment by
/// their identifiers, the younger first.
pub fn newest_kept_first(digests: &mut [Digest]) {
    digests.sort_by_key(|digest| Reverse((digest.kept_at(), digest.id())));
}

/// Groups found conversations by the folder they were held in, the folders
/// of the project first, then the other folders with the newest
/// conversation first; inside a folder the newest conversation first.
#[must_use]
pub fn group_by_folder(
    found: Vec<FoundConversation>,
    project_folders: &[PathBuf],
    kept: &HashSet<String>,
) -> Vec<ConversationsOfFolder> {
    let mut folders: Vec<ConversationsOfFolder> = Vec::new();
    for conversation in found {
        let card = card_of(conversation, kept);
        match folders
            .iter_mut()
            .find(|folder| folder.folder == card.held_in)
        {
            Some(folder) => folder.conversations.push(card),
            None => folders.push(ConversationsOfFolder {
                folder: card.held_in.clone(),
                is_of_project: lies_in_any(&card.held_in, project_folders),
                conversations: vec![card],
            }),
        }
    }
    for folder in &mut folders {
        folder.conversations.sort_by(|one, other| {
            newest(other)
                .cmp(&newest(one))
                .then_with(|| one.source.cmp(&other.source))
        });
    }
    folders.sort_by(|one, other| {
        other
            .is_of_project
            .cmp(&one.is_of_project)
            .then_with(|| newest_of_folder(other).cmp(&newest_of_folder(one)))
            .then_with(|| one.folder.cmp(&other.folder))
    });
    folders
}

/// When a conversation last changed, as far as it is known.
fn newest(card: &EarlierConversationCard) -> Option<Timestamp> {
    card.ended_at.or(card.started_at)
}

/// When the newest conversation of a folder last changed.
fn newest_of_folder(folder: &ConversationsOfFolder) -> Option<Timestamp> {
    folder.conversations.iter().filter_map(newest).max()
}

/// Whether a folder is one of the given folders, or lies inside one.
fn lies_in_any(folder: &str, folders: &[PathBuf]) -> bool {
    let folder = Path::new(folder);
    !folder.as_os_str().is_empty() && folders.iter().any(|known| folder.starts_with(known))
}

/// A found conversation as the list shows it.
fn card_of(conversation: FoundConversation, kept: &HashSet<String>) -> EarlierConversationCard {
    let shown = |text: &str, limit: usize| {
        let line = hide_secrets(text)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        (!line.is_empty()).then(|| cut(&line, limit))
    };
    EarlierConversationCard {
        is_kept: kept.contains(&conversation.source),
        title: conversation
            .title
            .as_deref()
            .and_then(|title| shown(title, DigestTitle::MAX_LENGTH)),
        first_request: conversation
            .first_request
            .as_deref()
            .and_then(|request| shown(request, FIRST_REQUEST_IN_LIST)),
        source: conversation.source,
        held_in: conversation.held_in,
        started_at: conversation.started_at,
        ended_at: conversation.ended_at,
        size_bytes: conversation.size_bytes,
    }
}

#[cfg(test)]
mod tests;
