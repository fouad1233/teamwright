//! Starts the program of a connector in a neutral folder, with only the
//! variables it was given
//! ([decision 0039](../../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md),
//! note of 21 November 2026).
//!
//! Claude Code starts the program of a connector itself, in the working
//! folder of the session, with the whole environment it was started in.
//! The working folder is the folder of the project, which the specialist
//! writes: a connector such as `python3 -m parts_server` would import a
//! `parts_server.py` the specialist wrote there. And the environment holds
//! whatever the manager's own does.
//!
//! So Claude Code is not given the program of a connector. It is given
//! Teamwright's own program, in the mode of this module, with the program
//! after it:
//!
//! ```text
//! teamwright --connector --folder <empty folder> [--secrets <file>]
//!            [--pass NAME]... -- <program> <arguments>...
//! ```
//!
//! The launcher clears the environment, sets the few variables every
//! program needs to start ([`BASE_VARIABLES`]) and the ones the connector
//! names, whose values it reads from a file of Teamwright's own that only
//! the user may read ([`SecretsFile`]), changes to the empty folder, and
//! then becomes the program: on Unix it replaces itself with it; on
//! Windows it starts it, hands it its input and output, and ends with its
//! exit code. It never writes to standard output, which is the channel of
//! the tool server; what goes wrong is said on standard error.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use teamwright_domain::{ConnectorStart, VariableName};

/// The first argument that starts this program as the launcher of a
/// connector.
pub const LAUNCHER_ARGUMENT: &str = "--connector";

/// The option that names the empty folder the program is started in.
const FOLDER_OPTION: &str = "--folder";

/// The option that names the file with the values of the variables.
const SECRETS_OPTION: &str = "--secrets";

/// The option that names one variable handed to the program.
const PASS_OPTION: &str = "--pass";

/// The argument after which the program and its own arguments come.
const PROGRAM_SEPARATOR: &str = "--";

/// The version of the format of the file of secrets.
const SECRETS_FORMAT_VERSION: u32 = 1;

/// The variables every program is started with, if the launcher has them:
/// what a program needs to start and to find its own files on each system.
/// Variables of the locale (`LC_…`) are added to these. Nothing else of
/// the environment Teamwright was started in reaches a connector, not even
/// a proxy: a connector that needs one names it like any other variable.
#[cfg(unix)]
pub const BASE_VARIABLES: &[&str] = &[
    "PATH", "HOME", "USER", "LOGNAME", "LANG", "LANGUAGE", "TZ", "TMPDIR",
];

/// The variables every program is started with, if the launcher has them:
/// what a program needs to start and to find its own files on each system.
/// Nothing else of the environment Teamwright was started in reaches a
/// connector, not even a proxy: a connector that needs one names it like
/// any other variable.
#[cfg(not(unix))]
pub const BASE_VARIABLES: &[&str] = &[
    "PATH",
    "PATHEXT",
    "SYSTEMROOT",
    "SYSTEMDRIVE",
    "WINDIR",
    "COMSPEC",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "USERNAME",
    "USERDOMAIN",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "PROGRAMW6432",
    "COMMONPROGRAMFILES",
    "COMMONPROGRAMFILES(X86)",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "OS",
];

/// Whether a variable is one every program is started with.
#[must_use]
pub fn is_base_variable(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    BASE_VARIABLES.contains(&upper.as_str()) || (cfg!(unix) && upper.starts_with("LC_"))
}

/// What the launcher is told: where to start, what to hand on, and what
/// to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The empty folder of Teamwright's own the program is started in.
    pub folder: PathBuf,
    /// The file with the values of the variables, if any is passed.
    pub secrets: Option<PathBuf>,
    /// The variables handed to the program, by name.
    pub variables: Vec<String>,
    /// The program: a bare name found on the search path, or a full path.
    pub program: String,
    /// Its arguments.
    pub arguments: Vec<String>,
}

impl Launch {
    /// The arguments after [`LAUNCHER_ARGUMENT`] that tell the launcher
    /// this, in the order [`Self::parse`] reads them.
    #[must_use]
    pub fn to_arguments(&self) -> Vec<String> {
        let mut arguments = vec![
            FOLDER_OPTION.to_owned(),
            self.folder.to_string_lossy().into_owned(),
        ];
        if let Some(secrets) = &self.secrets {
            arguments.push(SECRETS_OPTION.to_owned());
            arguments.push(secrets.to_string_lossy().into_owned());
        }
        for variable in &self.variables {
            arguments.push(PASS_OPTION.to_owned());
            arguments.push(variable.clone());
        }
        arguments.push(PROGRAM_SEPARATOR.to_owned());
        arguments.push(self.program.clone());
        arguments.extend(self.arguments.iter().cloned());
        arguments
    }

    /// Reads the arguments after [`LAUNCHER_ARGUMENT`].
    ///
    /// # Errors
    ///
    /// Returns what is wrong, in words: an option that is not known or has
    /// no value, a missing folder or program, a variable that is not a
    /// name or is passed while no file of secrets is named, or a program
    /// named by a relative path.
    pub fn parse(arguments: &[OsString]) -> Result<Self, String> {
        let mut texts = Vec::with_capacity(arguments.len());
        for argument in arguments {
            texts.push(
                argument
                    .to_str()
                    .ok_or("an argument is not text")?
                    .to_owned(),
            );
        }
        let mut folder = None;
        let mut secrets = None;
        let mut variables = Vec::new();
        let mut rest = texts.into_iter();
        loop {
            let Some(option) = rest.next() else {
                return Err("no program is named after --".to_owned());
            };
            if option == PROGRAM_SEPARATOR {
                break;
            }
            let value = rest
                .next()
                .ok_or_else(|| format!("the option {option} has no value"))?;
            match option.as_str() {
                FOLDER_OPTION if folder.is_none() => folder = Some(PathBuf::from(value)),
                SECRETS_OPTION if secrets.is_none() => secrets = Some(PathBuf::from(value)),
                PASS_OPTION => {
                    let name = VariableName::parse(&value).map_err(|error| error.to_string())?;
                    if variables.contains(&value) {
                        return Err(format!("the variable {name} is passed twice"));
                    }
                    variables.push(value);
                }
                _ => return Err(format!("the option {option} is not known here")),
            }
        }
        let folder = folder.ok_or("no folder is named to start in")?;
        if !variables.is_empty() && secrets.is_none() {
            return Err("variables are passed without a file that holds them".to_owned());
        }
        let program = rest.next().ok_or("no program is named after --")?;
        ConnectorStart::check_program_name(&program).map_err(|error| error.to_string())?;
        Ok(Self {
            folder,
            secrets,
            variables,
            program,
            arguments: rest.collect(),
        })
    }
}

/// The values of the variables of one connector, for one turn, in a file
/// of Teamwright's own that only the user may read, outside every folder
/// the session may read. The file is removed when this is dropped, at the
/// end of the turn.
#[derive(Debug)]
pub struct SecretsFile {
    path: PathBuf,
}

/// Counts the files of secrets this program writes, so that two turns that
/// start in the same instant get files of their own.
static WRITTEN: AtomicU64 = AtomicU64::new(0);

/// What a file of secrets holds.
#[derive(Debug, Serialize, Deserialize)]
struct SecretsContents {
    version: u32,
    variables: BTreeMap<String, String>,
}

impl SecretsFile {
    /// Writes the values to a new file in `folder`, which is made if it is
    /// missing, so that on Unix only the user may enter it.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be made or the file written.
    pub fn write_in(folder: &Path, values: &BTreeMap<String, String>) -> io::Result<Self> {
        make_private_folder(folder)?;
        let contents = serde_json::to_vec(&SecretsContents {
            version: SECRETS_FORMAT_VERSION,
            variables: values.clone(),
        })
        .map_err(io::Error::other)?;
        let (path, mut file) = new_private_file(folder)?;
        let written = file.write_all(&contents).and_then(|()| file.sync_all());
        let secrets = Self { path };
        written?;
        Ok(secrets)
    }

    /// Where the file is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SecretsFile {
    fn drop(&mut self) {
        // A failure here has nobody to be told to. A file left behind by a
        // crash is removed when Teamwright next starts.
        let _ = fs::remove_file(&self.path);
    }
}

/// Removes the files of secrets that a turn which never ended properly
/// left in `folder`. Called when Teamwright starts, before any turn.
pub fn remove_left_over_secrets(folder: &Path) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("teamwright-connector-") {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// The beginning of the name of every folder a connector starts in.
const START_FOLDER_PREFIX: &str = "teamwright-start-";

/// Counts the folders to start in this program makes, so that two that are
/// made in the same instant get names of their own.
static MADE: AtomicU64 = AtomicU64::new(0);

/// A fresh, empty folder of Teamwright's own that one connector of one turn
/// starts in, which only the user may enter (decision 0039, note of 22
/// November 2026). No other connector is started in it: a connector that
/// writes a file into its working folder cannot plant it for another one
/// (Python's `-m`, for one, looks there first). The folder is removed, with
/// whatever the connector wrote into it, when this is dropped, at the end
/// of the turn; one left by a turn that never ended properly is removed
/// when Teamwright next starts ([`remove_left_over_start_folders`]).
#[derive(Debug)]
pub struct StartFolder {
    path: PathBuf,
}

impl StartFolder {
    /// Makes a new empty folder inside `parent`, which is made if it is
    /// missing. Both may be entered by the user alone, on Unix.
    ///
    /// # Errors
    ///
    /// Returns an error if a folder cannot be made.
    pub fn make_in(parent: &Path) -> io::Result<Self> {
        make_private_folder(parent)?;
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        loop {
            let number = MADE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                "{START_FOLDER_PREFIX}{}-{started}-{number}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    let folder = Self { path };
                    make_private_folder(&folder.path)?;
                    return Ok(folder);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
    }

    /// Where the folder is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StartFolder {
    fn drop(&mut self) {
        // A failure here has nobody to be told to; what is left is removed
        // when Teamwright next starts. Links inside are removed, not
        // followed.
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Removes the folders to start in that a turn which never ended properly
/// left in `parent`. Called when Teamwright starts, before any turn.
pub fn remove_left_over_start_folders(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(START_FOLDER_PREFIX)
        {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Makes a folder, and on Unix lets only the user enter it.
///
/// # Errors
///
/// Returns an error if the folder cannot be made.
pub fn make_private_folder(folder: &Path) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(folder, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn new_private_file(folder: &Path) -> io::Result<(PathBuf, File)> {
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    loop {
        let number = WRITTEN.fetch_add(1, Ordering::Relaxed);
        let path = folder.join(format!(
            "teamwright-connector-{}-{started}-{number}.json",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
}

/// Reads the values of these variables from a file of secrets.
fn values_from(path: &Path, names: &[String]) -> Result<BTreeMap<String, String>, String> {
    let bytes = fs::read(path).map_err(|error| format!("the file of secrets: {error}"))?;
    let contents: SecretsContents = serde_json::from_slice(&bytes)
        .map_err(|error| format!("the file of secrets is not readable: {error}"))?;
    if contents.version != SECRETS_FORMAT_VERSION {
        return Err(format!(
            "the file of secrets has the unknown format version {}",
            contents.version
        ));
    }
    names
        .iter()
        .map(|name| {
            contents
                .variables
                .get(name)
                .map(|value| (name.clone(), value.clone()))
                .ok_or_else(|| format!("the file of secrets holds no value for {name}"))
        })
        .collect()
}

/// The environment the program is started with: the base variables the
/// launcher has, then the named ones.
#[must_use]
pub fn environment_for(
    own: impl IntoIterator<Item = (OsString, OsString)>,
    named: &BTreeMap<String, String>,
) -> BTreeMap<OsString, OsString> {
    let mut environment: BTreeMap<OsString, OsString> = own
        .into_iter()
        .filter(|(name, _)| is_base_variable(&name.to_string_lossy()))
        .collect();
    for (name, value) in named {
        environment.insert(OsString::from(name), OsString::from(value));
    }
    environment
}

/// Finds the program: a full path as it is, a bare name in the folders of
/// the search path that are full paths themselves. An empty or relative
/// entry of the search path would mean the current folder, and is skipped.
/// On Windows the extensions of `PATHEXT` are tried, as the command prompt
/// does.
#[must_use]
pub fn find_program(
    program: &str,
    search_path: Option<&OsStr>,
    extensions: &[String],
) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.is_absolute() {
        return is_program(path).then(|| path.to_owned());
    }
    if ConnectorStart::check_program_name(program).is_err() {
        return None;
    }
    let mut tried: Vec<String> = Vec::new();
    if extensions.is_empty() || path.extension().is_some() {
        tried.push(String::new());
    }
    tried.extend(extensions.iter().cloned());
    std::env::split_paths(search_path?)
        .filter(|folder| folder.is_absolute())
        .find_map(|folder| {
            tried
                .iter()
                .map(|extension| folder.join(format!("{program}{extension}")))
                .find(|candidate| is_program(candidate))
        })
}

/// Whether a path is a file that can be run: on Unix one that may be
/// executed, as a shell would skip any other on the search path.
fn is_program(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        metadata.is_file()
    }
}

/// The extensions a bare name is tried with: none on Unix, those of
/// `PATHEXT` on Windows.
fn program_extensions(pathext: Option<&OsStr>) -> Vec<String> {
    if cfg!(unix) {
        return Vec::new();
    }
    let listed = pathext.map_or_else(
        || ".COM;.EXE;.BAT;.CMD".to_owned(),
        |value| value.to_string_lossy().into_owned(),
    );
    listed
        .split(';')
        .map(str::trim)
        .filter(|extension| extension.starts_with('.'))
        .map(str::to_owned)
        .collect()
}

/// Whether a folder exists and is empty.
fn is_empty_folder(folder: &Path) -> bool {
    fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_none())
}

/// Starts the program of a connector as the arguments after
/// [`LAUNCHER_ARGUMENT`] say. On Unix this returns only if the program
/// could not be started; on Windows it returns the program's exit code.
///
/// # Errors
///
/// Returns what is wrong, in words, for standard error.
pub fn launch(arguments: &[OsString]) -> Result<i32, String> {
    let launch = Launch::parse(arguments)?;
    if !launch.folder.is_absolute() || !is_empty_folder(&launch.folder) {
        return Err(format!(
            "the folder {} to start in is not an empty folder named by its full path",
            launch.folder.display()
        ));
    }
    let named = match &launch.secrets {
        Some(file) => values_from(file, &launch.variables)?,
        None => BTreeMap::new(),
    };
    let environment = environment_for(std::env::vars_os(), &named);
    let search_path = environment.get(OsStr::new("PATH")).cloned().or_else(|| {
        environment
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("PATH"))
            .map(|(_, value)| value.clone())
    });
    let pathext = environment
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("PATHEXT"))
        .map(|(_, value)| value.clone());
    let program = find_program(
        &launch.program,
        search_path.as_deref(),
        &program_extensions(pathext.as_deref()),
    )
    .ok_or_else(|| {
        format!(
            "the program {} was not found as a full path or on the search path",
            launch.program
        )
    })?;
    let mut command = Command::new(&program);
    command
        .args(&launch.arguments)
        .env_clear()
        .envs(&environment)
        .current_dir(&launch.folder);
    become_or_wait_for(command, &program)
}

#[cfg(unix)]
fn become_or_wait_for(mut command: Command, program: &Path) -> Result<i32, String> {
    use std::os::unix::process::CommandExt;
    // Only returns if the program could not be started.
    let error = command.exec();
    Err(format!(
        "the program {} could not be started: {error}",
        program.display()
    ))
}

#[cfg(not(unix))]
fn become_or_wait_for(mut command: Command, program: &Path) -> Result<i32, String> {
    // Input, output and errors are this process's own, handed on as they
    // are; the exit code is passed back.
    let status = command.status().map_err(|error| {
        format!(
            "the program {} could not be started: {error}",
            program.display()
        )
    })?;
    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(arguments: &[&str]) -> Vec<OsString> {
        arguments.iter().map(OsString::from).collect()
    }

    fn sample() -> Launch {
        Launch {
            folder: PathBuf::from("/data/connectors/start-here"),
            secrets: Some(PathBuf::from("/data/connectors/secrets/t.json")),
            variables: vec!["PARTS_TOKEN".to_owned()],
            program: "npx".to_owned(),
            arguments: vec!["-y".to_owned(), "parts-server".to_owned(), "--".to_owned()],
        }
    }

    #[test]
    fn the_arguments_are_exactly_these_and_read_back_the_same() {
        let launch = sample();
        let arguments = launch.to_arguments();
        assert_eq!(
            arguments,
            [
                "--folder",
                "/data/connectors/start-here",
                "--secrets",
                "/data/connectors/secrets/t.json",
                "--pass",
                "PARTS_TOKEN",
                "--",
                "npx",
                "-y",
                "parts-server",
                "--",
            ]
        );
        let back = Launch::parse(&arguments.iter().map(OsString::from).collect::<Vec<_>>());
        assert_eq!(back, Ok(launch));
    }

    #[test]
    fn a_launch_without_variables_names_no_file() {
        let mut launch = sample();
        launch.secrets = None;
        launch.variables = Vec::new();
        assert_eq!(
            launch.to_arguments()[..3],
            ["--folder", "/data/connectors/start-here", "--"]
        );
    }

    #[test]
    fn what_the_launcher_does_not_understand_is_refused() {
        for (arguments, expected) in [
            (vec!["--folder", "/x"], "no program"),
            (vec!["--folder"], "has no value"),
            (vec!["--", "npx"], "no folder"),
            (
                vec!["--folder", "/x", "--pass", "A", "--", "npx"],
                "without a file",
            ),
            (
                vec!["--folder", "/x", "--env", "A=1", "--", "npx"],
                "not known",
            ),
            (
                vec!["--folder", "/x", "--folder", "/y", "--", "npx"],
                "not known",
            ),
            (vec!["--folder", "/x", "--", "./server"], "relative path"),
            (vec!["--folder", "/x", "--", "bin/server"], "relative path"),
            (
                vec![
                    "--folder",
                    "/x",
                    "--secrets",
                    "/s",
                    "--pass",
                    "1A",
                    "--",
                    "npx",
                ],
                "not the name",
            ),
        ] {
            let refusal = Launch::parse(&texts(&arguments)).unwrap_err();
            assert!(refusal.contains(expected), "{arguments:?}: {refusal}");
        }
    }

    #[test]
    fn the_environment_is_the_base_and_the_named_variables_only() {
        let own = [
            ("PATH", "/usr/bin"),
            ("HOME", "/home/sample"),
            ("AWS_SECRET_ACCESS_KEY", "never"),
            ("PYTHONPATH", "/home/sample/project"),
            ("NODE_OPTIONS", "--require ./evil.js"),
            ("HTTPS_PROXY", "http://user:pass@proxy"),
        ]
        .map(|(name, value)| (OsString::from(name), OsString::from(value)));
        let named = BTreeMap::from([("PARTS_TOKEN".to_owned(), "t0ken".to_owned())]);

        let environment = environment_for(own, &named);

        let names: Vec<String> = environment
            .keys()
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"PATH".to_owned()));
        assert!(names.contains(&"PARTS_TOKEN".to_owned()));
        for never in [
            "AWS_SECRET_ACCESS_KEY",
            "PYTHONPATH",
            "NODE_OPTIONS",
            "HTTPS_PROXY",
        ] {
            assert!(!names.contains(&never.to_owned()), "{never}");
        }
    }

    #[test]
    fn a_secrets_file_holds_the_values_for_the_user_alone_and_goes_with_the_turn() {
        let ground = tempfile::tempdir().unwrap();
        let folder = ground.path().join("secrets");
        let values = BTreeMap::from([("PARTS_TOKEN".to_owned(), "t0ken".to_owned())]);
        let file = SecretsFile::write_in(&folder, &values).unwrap();
        let path = file.path().to_owned();

        assert_eq!(
            values_from(&path, &["PARTS_TOKEN".to_owned()]).unwrap(),
            values
        );
        assert!(
            values_from(&path, &["OTHER".to_owned()])
                .unwrap_err()
                .contains("OTHER")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
            let mode = fs::metadata(&folder).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
        }
        drop(file);
        assert!(!path.exists());

        let left = SecretsFile::write_in(&folder, &values).unwrap();
        let left_path = left.path().to_owned();
        std::mem::forget(left);
        remove_left_over_secrets(&folder);
        assert!(!left_path.exists());
    }

    #[test]
    fn each_start_gets_a_fresh_empty_folder_of_its_own_gone_with_the_turn() {
        let ground = tempfile::tempdir().unwrap();
        let parent = ground.path().join("starts");

        let first = StartFolder::make_in(&parent).unwrap();
        let second = StartFolder::make_in(&parent).unwrap();

        assert_ne!(first.path(), second.path());
        assert!(first.path().starts_with(&parent) && second.path().starts_with(&parent));
        assert!(is_empty_folder(first.path()) && is_empty_folder(second.path()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for folder in [&parent, &first.path().to_owned()] {
                let mode = fs::metadata(folder).unwrap().permissions().mode();
                assert_eq!(mode & 0o777, 0o700);
            }
        }
        // What one connector writes stays in its folder and goes with it.
        fs::write(first.path().join("parts_server.py"), "planted").unwrap();
        assert!(is_empty_folder(second.path()));
        let first_path = first.path().to_owned();
        drop(first);
        assert!(!first_path.exists());

        // A folder a turn that never ended left behind goes at the next
        // start, and nothing else in the parent does.
        let left = second.path().to_owned();
        fs::write(left.join("module.py"), "x").unwrap();
        std::mem::forget(second);
        fs::write(parent.join("kept.txt"), "kept").unwrap();
        remove_left_over_start_folders(&parent);
        assert!(!left.exists());
        assert!(parent.join("kept.txt").exists());
    }

    fn make_program(path: &Path) {
        fs::write(path, "").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn a_program_is_found_on_the_full_folders_of_the_search_path_only() {
        let ground = tempfile::tempdir().unwrap();
        let bin = ground.path().join("bin");
        fs::create_dir(&bin).unwrap();
        make_program(&bin.join("parts-server"));
        // A file that may not be run is passed over, as a shell does.
        let other = ground.path().join("other");
        fs::create_dir(&other).unwrap();
        fs::write(other.join("parts-server"), "").unwrap();
        let search =
            std::env::join_paths([PathBuf::from("."), other.clone(), bin.clone()]).unwrap();

        assert_eq!(
            find_program("parts-server", Some(&search), &[]),
            Some(bin.join("parts-server"))
        );
        assert_eq!(find_program("missing", Some(&search), &[]), None);
        assert_eq!(find_program("parts-server", None, &[]), None);
        // A relative entry of the search path is not looked in.
        let relative = std::env::join_paths([PathBuf::from("bin")]).unwrap();
        assert_eq!(find_program("parts-server", Some(&relative), &[]), None);
        // A full path is taken as it is; a relative one never.
        let full = bin.join("parts-server");
        assert_eq!(
            find_program(&full.to_string_lossy(), None, &[]),
            Some(full.clone())
        );
        assert_eq!(find_program("bin/parts-server", Some(&search), &[]), None);
        // Extensions are tried as on Windows.
        make_program(&bin.join("tool.cmd"));
        assert_eq!(
            find_program(
                "tool",
                Some(&search),
                &[".exe".to_owned(), ".cmd".to_owned()]
            ),
            Some(bin.join("tool.cmd"))
        );
    }

    #[test]
    fn the_launcher_starts_nothing_outside_an_empty_folder_named_in_full() {
        let ground = tempfile::tempdir().unwrap();
        fs::write(ground.path().join("parts_server.py"), "").unwrap();
        let busy = ground.path().to_string_lossy().into_owned();
        let refusal = launch(&texts(&["--folder", &busy, "--", "npx"])).unwrap_err();
        assert!(refusal.contains("empty folder"), "{refusal}");
        let refusal = launch(&texts(&["--folder", "relative", "--", "npx"])).unwrap_err();
        assert!(refusal.contains("empty folder"), "{refusal}");
    }
}
