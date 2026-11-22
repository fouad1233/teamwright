//! The shelf of skills on this computer: the copies of the folders the
//! manager added as skills, each in the form Claude Code loads for one
//! session ([decision 0039](../../../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md)).
//!
//! A copy lies in `skills/<identifier of the skill>/` in the data folder of
//! Teamwright: `.claude-plugin/plugin.json`, which names it, and
//! `skills/<name>/`, the folder of the skill. Nothing else is ever put
//! there, so that a session loads nothing but the skill. The folder the
//! manager chose is only read; links in it are refused, never followed.
//!
//! Before every turn the copy is read again: what lies at its top, in the
//! folder of the manifest and in the folder of skills, whether it holds a
//! link, and how large it is. The application decides from that whether
//! the copy is still exactly what Teamwright wrote. The limits of size
//! hold while the copy is made, not only when the folder is measured
//! before.

use std::fs::{self, File, Metadata};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use teamwright_application::{
    KeptLayout, KeptSkill, MANIFEST_FILE, MANIFEST_FOLDER, MAX_SKILL_BYTES, MAX_SKILL_FILES,
    SKILL_FILE, SKILLS_FOLDER, SkillFolder, SkillShelf, StorageError,
};
use teamwright_domain::{Skill, SkillId, VariableName};

/// The largest `SKILL.md` that is read: 1 MiB.
const MAX_SKILL_FILE_BYTES: u64 = 1024 * 1024;

/// The shelf in a folder of this computer, and the environment of this
/// process.
pub struct LocalSkillShelf {
    root: PathBuf,
}

impl LocalSkillShelf {
    /// The shelf in this folder. It is created when the first skill is kept.
    pub const fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn folder_of(&self, skill: SkillId) -> PathBuf {
        self.root.join(skill.to_string())
    }
}

fn failed(what: String) -> impl FnOnce(io::Error) -> StorageError {
    move |error| StorageError::caused_by(what, error)
}

/// Counts the files of a folder and their bytes, and notes links, without
/// following any. Counting stops a little past the limits.
fn measure(folder: &Path, found: &mut SkillFolder) -> Result<(), StorageError> {
    let entries = fs::read_dir(folder).map_err(failed(format!(
        "the folder {} could not be read",
        folder.display()
    )))?;
    for entry in entries {
        if found.files > MAX_SKILL_FILES || found.bytes > MAX_SKILL_BYTES || found.has_links {
            return Ok(());
        }
        let entry = entry.map_err(failed(format!(
            "the folder {} could not be read",
            folder.display()
        )))?;
        let kind = entry.file_type().map_err(failed(format!(
            "{} could not be read",
            entry.path().display()
        )))?;
        if kind.is_symlink() {
            found.has_links = true;
        } else if kind.is_dir() {
            measure(&entry.path(), found)?;
        } else {
            found.files += 1;
            found.bytes += entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
        }
    }
    Ok(())
}

/// What a copy may still take: files and bytes.
struct Allowance {
    files: usize,
    bytes: u64,
}

impl Allowance {
    const fn of_a_skill() -> Self {
        Self {
            files: MAX_SKILL_FILES,
            bytes: MAX_SKILL_BYTES,
        }
    }
}

fn too_large() -> StorageError {
    StorageError::new(format!(
        "the folder is larger than a skill may be: at most {MAX_SKILL_FILES} files and {} MiB",
        MAX_SKILL_BYTES / 1024 / 1024
    ))
}

/// Whether two descriptions are of the same file. On Unix by its device and
/// number, so that a file swapped for a link after it was looked at is
/// noticed; elsewhere only that both are plain files.
#[cfg(unix)]
fn same_file(looked_at: &Metadata, opened: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    looked_at.dev() == opened.dev() && looked_at.ino() == opened.ino()
}

#[cfg(not(unix))]
fn same_file(looked_at: &Metadata, opened: &Metadata) -> bool {
    looked_at.is_file() && opened.is_file()
}

/// Copies one plain file within what the copy may still take, never
/// through a link.
fn copy_file(from: &Path, to: &Path, allowance: &mut Allowance) -> Result<(), StorageError> {
    if allowance.files == 0 {
        return Err(too_large());
    }
    allowance.files -= 1;
    let looked_at = fs::symlink_metadata(from)
        .map_err(failed(format!("{} could not be read", from.display())))?;
    let source =
        File::open(from).map_err(failed(format!("{} could not be read", from.display())))?;
    let opened = source
        .metadata()
        .map_err(failed(format!("{} could not be read", from.display())))?;
    if !looked_at.is_file() || !opened.is_file() || !same_file(&looked_at, &opened) {
        return Err(StorageError::new(format!(
            "{} changed while it was copied, or is a link, and a skill may not hold one",
            from.display()
        )));
    }
    let mut target =
        File::create(to).map_err(failed(format!("{} could not be written", to.display())))?;
    // One byte more than allowed is read, to know that the file is larger.
    let copied = io::copy(&mut source.take(allowance.bytes + 1), &mut target)
        .map_err(failed(format!("{} could not be copied", from.display())))?;
    if copied > allowance.bytes {
        return Err(too_large());
    }
    allowance.bytes -= copied;
    Ok(())
}

/// Copies a folder, its files and its folders, refusing a link, within
/// what a skill may hold.
fn copy_folder(from: &Path, to: &Path, allowance: &mut Allowance) -> Result<(), StorageError> {
    fs::create_dir_all(to).map_err(failed(format!(
        "the folder {} could not be made",
        to.display()
    )))?;
    let entries = fs::read_dir(from).map_err(failed(format!(
        "the folder {} could not be read",
        from.display()
    )))?;
    for entry in entries {
        let entry = entry.map_err(failed(format!(
            "the folder {} could not be read",
            from.display()
        )))?;
        let kind = entry.file_type().map_err(failed(format!(
            "{} could not be read",
            entry.path().display()
        )))?;
        let target = to.join(entry.file_name());
        if kind.is_symlink() {
            return Err(StorageError::new(format!(
                "{} is a link, and a skill may not hold one",
                entry.path().display()
            )));
        } else if kind.is_dir() {
            copy_folder(&entry.path(), &target, allowance)?;
        } else {
            copy_file(&entry.path(), &target, allowance)?;
        }
    }
    Ok(())
}

/// Whether a folder holds a link anywhere, without following one. Also
/// true if it cannot be read, or holds more entries than are left to look
/// at.
fn holds_a_link(folder: &Path, entries_left: &mut usize) -> bool {
    let Ok(entries) = fs::read_dir(folder) else {
        return true;
    };
    for entry in entries {
        let Some(left) = entries_left.checked_sub(1) else {
            return true;
        };
        *entries_left = left;
        let Ok((kind, path)) =
            entry.and_then(|entry| entry.file_type().map(|kind| (kind, entry.path())))
        else {
            return true;
        };
        if kind.is_symlink() || (kind.is_dir() && holds_a_link(&path, entries_left)) {
            return true;
        }
    }
    false
}

/// The names in a folder, without following a link; none if it cannot be
/// read or is not a folder.
fn names_in(folder: &Path) -> Vec<String> {
    let is_folder = fs::symlink_metadata(folder).is_ok_and(|metadata| metadata.is_dir());
    if !is_folder {
        return Vec::new();
    }
    let mut names: Vec<String> = fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The text of a `SKILL.md` that is a plain file of a sensible size.
fn read_skill_file(path: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_SKILL_FILE_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

impl SkillShelf for LocalSkillShelf {
    fn read_folder(&self, folder: &Path) -> Result<SkillFolder, StorageError> {
        let metadata = fs::symlink_metadata(folder).map_err(failed(format!(
            "the folder {} could not be read",
            folder.display()
        )))?;
        if !metadata.is_dir() {
            return Err(StorageError::new(format!(
                "{} is not a folder",
                folder.display()
            )));
        }
        let mut found = SkillFolder {
            folder_name: folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            skill_text: read_skill_file(&folder.join(SKILL_FILE)),
            files: 0,
            bytes: 0,
            has_links: false,
        };
        measure(folder, &mut found)?;
        Ok(found)
    }

    fn keep(&self, from: &Path, skill: &Skill) -> Result<(), StorageError> {
        let target = self.folder_of(skill.id());
        let fresh = self.root.join(format!("{}.new", skill.id()));
        if fresh.exists() {
            fs::remove_dir_all(&fresh)
                .map_err(failed(format!("{} could not be cleared", fresh.display())))?;
        }
        let manifest = fresh.join(MANIFEST_FOLDER);
        fs::create_dir_all(&manifest).map_err(failed(format!(
            "the folder {} could not be made",
            manifest.display()
        )))?;
        let named = serde_json::json!({
            "name": skill.name().as_str(),
            "description": skill.description(),
        });
        fs::write(manifest.join(MANIFEST_FILE), named.to_string()).map_err(failed(format!(
            "the manifest in {} could not be written",
            manifest.display()
        )))?;
        let copied = copy_folder(
            from,
            &fresh.join(SKILLS_FOLDER).join(skill.name().as_str()),
            &mut Allowance::of_a_skill(),
        );
        if let Err(error) = copied {
            let _ = fs::remove_dir_all(&fresh);
            return Err(error);
        }
        if target.exists() {
            fs::remove_dir_all(&target).map_err(failed(format!(
                "{} could not be replaced",
                target.display()
            )))?;
        }
        fs::rename(&fresh, &target).map_err(failed(format!(
            "{} could not be put in place",
            target.display()
        )))
    }

    fn kept(&self, skill: SkillId) -> Option<KeptSkill> {
        let folder = self.folder_of(skill);
        let top = fs::symlink_metadata(&folder).ok()?;
        let mut layout = KeptLayout {
            has_links: top.is_symlink(),
            ..KeptLayout::default()
        };
        if !layout.has_links {
            layout.top = names_in(&folder);
            layout.manifest = names_in(&folder.join(MANIFEST_FOLDER));
            layout.skills = names_in(&folder.join(SKILLS_FOLDER));
            // A link anywhere, also beside the skill. A copy too large or
            // too strange to be looked through whole is taken as one.
            let mut entries_left = 4 * MAX_SKILL_FILES;
            layout.has_links = holds_a_link(&folder, &mut entries_left);
            let mut of_the_skill = SkillFolder {
                folder_name: String::new(),
                skill_text: None,
                files: 0,
                bytes: 0,
                has_links: false,
            };
            if measure(&folder.join(SKILLS_FOLDER), &mut of_the_skill).is_err() {
                layout.has_links = true;
            }
            layout.files = of_the_skill.files;
            layout.bytes = of_the_skill.bytes;
        }
        let skill_folder_name = match layout.skills.as_slice() {
            [only] => only.clone(),
            _ => String::new(),
        };
        let skill_text = if skill_folder_name.is_empty() || layout.has_links {
            String::new()
        } else {
            read_skill_file(
                &folder
                    .join(SKILLS_FOLDER)
                    .join(&skill_folder_name)
                    .join(SKILL_FILE),
            )
            .unwrap_or_default()
        };
        Some(KeptSkill {
            folder,
            skill_folder_name,
            skill_text,
            layout,
        })
    }

    fn forget(&self, skill: SkillId) -> Result<(), StorageError> {
        let folder = self.folder_of(skill);
        match fs::remove_dir_all(&folder) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(StorageError::caused_by(
                format!("{} could not be removed", folder.display()),
                error,
            )),
            _ => Ok(()),
        }
    }

    fn has_variable(&self, name: &VariableName) -> bool {
        std::env::var_os(name.as_str()).is_some_and(|value| !value.is_empty())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a failed test is the right outcome")]

    use teamwright_domain::{ProjectId, SkillFile};

    use super::*;

    const REVIEW: &str = "---\nname: pcb-review\ndescription: Reviews a layout.\n---\nLook.\n";

    fn skill_folder(ground: &Path) -> PathBuf {
        let folder = ground.join("pcb-review");
        fs::create_dir_all(folder.join("checklists")).unwrap();
        fs::write(folder.join(SKILL_FILE), REVIEW).unwrap();
        fs::write(folder.join("checklists").join("layers.md"), "Four layers.").unwrap();
        folder
    }

    #[test]
    fn a_folder_is_measured_and_kept_in_the_form_of_a_plugin() {
        let ground = tempfile::tempdir().unwrap();
        let shelf = LocalSkillShelf::new(ground.path().join("shelf"));
        let folder = skill_folder(ground.path());

        let found = shelf.read_folder(&folder).unwrap();
        assert_eq!(found.folder_name, "pcb-review");
        assert_eq!(found.skill_text.as_deref(), Some(REVIEW));
        assert_eq!(found.files, 2);
        assert!(!found.has_links);

        let skill = Skill::add(
            ProjectId::generate(),
            SkillFile::check(REVIEW, "pcb-review").unwrap(),
            chrono::Utc::now(),
        );
        shelf.keep(&folder, &skill).unwrap();
        let kept = shelf.kept(skill.id()).unwrap();
        assert_eq!(kept.skill_folder_name, "pcb-review");
        assert_eq!(kept.skill_text, REVIEW);
        let mut top: Vec<String> = fs::read_dir(&kept.folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        top.sort();
        assert_eq!(top, [".claude-plugin", "skills"]);
        let manifest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(kept.folder.join(".claude-plugin").join("plugin.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["name"], "pcb-review");
        assert!(
            kept.folder
                .join("skills/pcb-review/checklists/layers.md")
                .is_file()
        );

        // Kept again, the copy is replaced; forgotten, it is gone, and the
        // folder of the manager is untouched.
        shelf.keep(&folder, &skill).unwrap();
        shelf.forget(skill.id()).unwrap();
        assert!(shelf.kept(skill.id()).is_none());
        shelf.forget(skill.id()).unwrap();
        assert!(folder.join(SKILL_FILE).is_file());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_in_a_folder_is_noted_and_never_copied() {
        let ground = tempfile::tempdir().unwrap();
        let shelf = LocalSkillShelf::new(ground.path().join("shelf"));
        let folder = skill_folder(ground.path());
        std::os::unix::fs::symlink("/etc", folder.join("outside")).unwrap();

        assert!(shelf.read_folder(&folder).unwrap().has_links);
        let skill = Skill::add(
            ProjectId::generate(),
            SkillFile::check(REVIEW, "pcb-review").unwrap(),
            chrono::Utc::now(),
        );
        assert!(shelf.keep(&folder, &skill).is_err());
        assert!(shelf.kept(skill.id()).is_none());
    }

    fn review_skill() -> Skill {
        Skill::add(
            ProjectId::generate(),
            SkillFile::check(REVIEW, "pcb-review").unwrap(),
            chrono::Utc::now(),
        )
    }

    #[test]
    fn a_kept_copy_is_read_again_whole_and_what_was_added_to_it_is_seen() {
        let ground = tempfile::tempdir().unwrap();
        let shelf = LocalSkillShelf::new(ground.path().join("shelf"));
        let folder = skill_folder(ground.path());
        let skill = review_skill();
        shelf.keep(&folder, &skill).unwrap();

        let kept = shelf.kept(skill.id()).unwrap();
        assert_eq!(kept.layout.top, [MANIFEST_FOLDER, SKILLS_FOLDER]);
        assert_eq!(kept.layout.manifest, [MANIFEST_FILE]);
        assert_eq!(kept.layout.skills, ["pcb-review"]);
        assert_eq!(kept.layout.files, 2);
        assert!(!kept.layout.has_links);
        assert_eq!(
            teamwright_application::check_kept_layout(&kept.layout, skill.name()),
            Ok(())
        );

        // What Claude Code would load from a plugin besides its skills.
        fs::create_dir(kept.folder.join("hooks")).unwrap();
        fs::write(kept.folder.join("hooks").join("hooks.json"), "{}").unwrap();
        fs::write(kept.folder.join(".mcp.json"), "{}").unwrap();
        let changed = shelf.kept(skill.id()).unwrap();
        assert_eq!(
            changed.layout.top,
            [".claude-plugin", ".mcp.json", "hooks", "skills"]
        );
        let problem =
            teamwright_application::check_kept_layout(&changed.layout, skill.name()).unwrap_err();
        assert!(problem.contains(".mcp.json"), "{problem}");
    }

    #[cfg(unix)]
    #[test]
    fn a_link_put_into_a_kept_copy_is_seen() {
        let ground = tempfile::tempdir().unwrap();
        let shelf = LocalSkillShelf::new(ground.path().join("shelf"));
        let folder = skill_folder(ground.path());
        let skill = review_skill();
        shelf.keep(&folder, &skill).unwrap();
        let kept = shelf.kept(skill.id()).unwrap();
        std::os::unix::fs::symlink(
            "/etc/hostname",
            kept.folder
                .join(SKILLS_FOLDER)
                .join("pcb-review")
                .join("checklists")
                .join("host.md"),
        )
        .unwrap();

        assert!(shelf.kept(skill.id()).unwrap().layout.has_links);
    }

    #[test]
    fn the_limits_hold_while_a_folder_is_copied() {
        let ground = tempfile::tempdir().unwrap();
        let shelf = LocalSkillShelf::new(ground.path().join("shelf"));
        let skill = review_skill();

        let large = skill_folder(ground.path());
        let bytes = usize::try_from(MAX_SKILL_BYTES).unwrap();
        fs::write(large.join("large.bin"), vec![0_u8; bytes]).unwrap();
        let refusal = shelf.keep(&large, &skill).unwrap_err().to_string();
        assert!(refusal.contains("larger than a skill may be"), "{refusal}");
        assert!(shelf.kept(skill.id()).is_none());
        fs::remove_file(large.join("large.bin")).unwrap();

        for number in 0..MAX_SKILL_FILES {
            fs::write(large.join(format!("note-{number}.md")), "x").unwrap();
        }
        let refusal = shelf.keep(&large, &skill).unwrap_err().to_string();
        assert!(refusal.contains("larger than a skill may be"), "{refusal}");
        assert!(shelf.kept(skill.id()).is_none());
    }

    #[test]
    fn a_file_is_not_a_folder_and_a_missing_skill_file_is_none() {
        let ground = tempfile::tempdir().unwrap();
        let shelf = LocalSkillShelf::new(ground.path().join("shelf"));
        fs::write(ground.path().join("note.txt"), "x").unwrap();
        assert!(shelf.read_folder(&ground.path().join("note.txt")).is_err());
        fs::create_dir(ground.path().join("empty")).unwrap();
        assert_eq!(
            shelf
                .read_folder(&ground.path().join("empty"))
                .unwrap()
                .skill_text,
            None
        );
    }

    #[test]
    fn a_variable_is_set_only_with_a_value() {
        let shelf = LocalSkillShelf::new(PathBuf::from("unused"));
        let path = VariableName::parse("PATH").unwrap();
        assert!(shelf.has_variable(&path));
        let absent = VariableName::parse("TEAMWRIGHT_SURELY_NOT_SET_ANYWHERE").unwrap();
        assert!(!shelf.has_variable(&absent));
    }
}
