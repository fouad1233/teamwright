//! The shelf of skills on this computer: the copies of the folders the
//! manager added as skills, each in the form Claude Code loads for one
//! session ([decision 0039](../../../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md)).
//!
//! A copy lies in `skills/<identifier of the skill>/` in the data folder of
//! Teamwright: `.claude-plugin/plugin.json`, which names it, and
//! `skills/<name>/`, the folder of the skill. Nothing else is ever put
//! there, so that a session loads nothing but the skill. The folder the
//! manager chose is only read; links in it are refused, never followed.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use teamwright_application::{
    KeptSkill, MAX_SKILL_BYTES, MAX_SKILL_FILES, SKILL_FILE, SkillFolder, SkillShelf, StorageError,
};
use teamwright_domain::{Skill, SkillId, VariableName};

/// The folder with the manifest of a plugin, as Claude Code expects it.
const MANIFEST_FOLDER: &str = ".claude-plugin";

/// The folder of the skills of a plugin.
const SKILLS_FOLDER: &str = "skills";

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

/// Copies a folder, its files and its folders, refusing a link.
fn copy_folder(from: &Path, to: &Path) -> Result<(), StorageError> {
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
            copy_folder(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(failed(format!(
                "{} could not be copied",
                entry.path().display()
            )))?;
        }
    }
    Ok(())
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
        fs::write(manifest.join("plugin.json"), named.to_string()).map_err(failed(format!(
            "the manifest in {} could not be written",
            manifest.display()
        )))?;
        let copied = copy_folder(from, &fresh.join(SKILLS_FOLDER).join(skill.name().as_str()));
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
        let mut names = fs::read_dir(folder.join(SKILLS_FOLDER))
            .ok()?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.file_name().to_string_lossy().into_owned());
        let name = names.next()?;
        if names.next().is_some() {
            return None;
        }
        let skill_text = read_skill_file(&folder.join(SKILLS_FOLDER).join(&name).join(SKILL_FILE))?;
        Some(KeptSkill {
            folder,
            skill_folder_name: name,
            skill_text,
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
