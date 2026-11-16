//! Opening a web address in the manager's own browser.
//!
//! The address comes from the interface, so it is checked before anything
//! sees it: only `https` with a host is opened, and what is handed to the
//! system is the address as the parser wrote it again, never the text that
//! came in. It goes to the system's own opener as one argument, without a
//! shell, so that nothing in it can be read as a command.

use std::process::{Command, Stdio};
use std::thread;

use tauri::Url;

use crate::error::CommandError;

/// Opens a web address in the browser the manager chose for the computer.
/// Anything but `https` with a host is refused (`invalid_request`).
#[tauri::command]
pub async fn open_web_address(address: String) -> Result<(), CommandError> {
    let address = checked_web_address(&address)?;
    let mut opener = open_command(address.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            CommandError::not_available(format!("the browser could not be started: {error}"))
        })?;
    // The command does not wait for the browser. A thread waits for the
    // opener instead, so that it does not linger as a finished process.
    thread::spawn(move || {
        let _ = opener.wait();
    });
    Ok(())
}

/// The address to open, if it is one: `https`, with a host, and nothing in
/// its written form that the opener of the system could read otherwise.
pub fn checked_web_address(text: &str) -> Result<Url, CommandError> {
    let refused = |why: &str| CommandError::invalid_request(format!("{why}: {}", text.trim()));
    let address = Url::parse(text.trim()).map_err(|_| refused("this is not a web address"))?;
    if address.scheme() != "https" {
        return Err(refused(
            "only an address that starts with https:// is opened",
        ));
    }
    if address.host_str().is_none_or(str::is_empty) {
        return Err(refused("the address names no host"));
    }
    if !address.username().is_empty() || address.password().is_some() {
        return Err(refused(
            "an address with a name and a password is not opened",
        ));
    }
    // The parser writes spaces, quotation marks and control characters
    // escaped; this makes sure of it, because the opener of Windows reads
    // its arguments in its own way.
    let written = address.as_str();
    if written
        .chars()
        .any(|character| character.is_whitespace() || character.is_control() || character == '"')
    {
        return Err(refused(
            "the address holds characters that cannot be opened",
        ));
    }
    Ok(address)
}

/// The command that opens a web address in the browser of the system.
#[cfg(target_os = "windows")]
fn open_command(address: &str) -> Command {
    use std::os::windows::process::CommandExt;

    let mut command = Command::new("explorer");
    // Explorer splits its arguments at commas unless they are quoted. The
    // checked address holds no quotation mark, so it cannot break out.
    command.raw_arg(format!("\"{address}\""));
    command
}

/// The command that opens a web address in the browser of the system.
#[cfg(target_os = "macos")]
fn open_command(address: &str) -> Command {
    let mut command = Command::new("open");
    command.arg(address);
    command
}

/// The command that opens a web address in the browser of the system.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn open_command(address: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(address);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened(text: &str) -> Option<String> {
        checked_web_address(text).ok().map(String::from)
    }

    #[test]
    fn an_https_address_with_a_host_is_opened_as_the_parser_writes_it() {
        assert_eq!(
            opened(" https://github.com/teamwright/teamwright/pull/7 ").as_deref(),
            Some("https://github.com/teamwright/teamwright/pull/7")
        );
        assert_eq!(
            opened("https://example.com/a b?q=\"x\",y").as_deref(),
            Some("https://example.com/a%20b?q=%22x%22,y")
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for text in [
            "",
            "github.com",
            "http://example.com",
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "ms-settings:privacy",
            "https://",
            "https://user:secret@example.com",
            "\\\\server\\share",
        ] {
            assert_eq!(opened(text), None, "{text}");
        }
    }
}
