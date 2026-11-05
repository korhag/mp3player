// ------------------------------------------------------------
// Module: update
// Purpose: Notices when the project folder has a newer commit than
//          the running program, and starts the installer that
//          refreshes the menu and shortcuts.
// ------------------------------------------------------------

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config;

/// Commit baked in when this binary was compiled.
pub const GIT_SHA: &str = env!("SOUNDKEEP_GIT_SHA");

/// Project folder this binary was compiled from.
const SOURCE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

// ------------------------------------------------------------
// Struct: Offer
// Purpose: What the install prompt needs to name both copies.
// ------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub source_sha: String,
    pub source_version: String,
    pub running_version: String,
}

// ------------------------------------------------------------
// Function: look_for_update
// Purpose: Compares this binary with the local checkout. Nothing
//          is fetched from the network.
// ------------------------------------------------------------
pub fn look_for_update(dismissed_sha: &str) -> Option<Offer> {
    let root = Path::new(SOURCE_ROOT);
    let ready = checkout_ready(root);
    let source_sha = git_head(root).unwrap_or_default();
    if !should_offer(GIT_SHA, &source_sha, dismissed_sha, ready) {
        return None;
    }
    let source_version = std::fs::read_to_string(root.join("Cargo.toml"))
        .ok()
        .and_then(|text| cargo_package_version(&text))
        .unwrap_or_default();
    Some(Offer {
        source_sha,
        source_version,
        running_version: config::VERSION.to_string(),
    })
}

// ------------------------------------------------------------
// Function: should_offer
// Purpose: True when the checkout commit differs from this binary
//          and the person has not already dismissed that commit.
// ------------------------------------------------------------
pub fn should_offer(
    running_sha: &str,
    source_sha: &str,
    dismissed_sha: &str,
    checkout_ready: bool,
) -> bool {
    checkout_ready
        && is_commit(running_sha)
        && is_commit(source_sha)
        && running_sha != source_sha
        && dismissed_sha != source_sha
}

// ------------------------------------------------------------
// Function: prompt_body
// Purpose: The sentence shown on the install card.
// ------------------------------------------------------------
pub fn prompt_body(running_version: &str, source_version: &str) -> String {
    let intro = if source_version.is_empty() || source_version == running_version {
        format!("Your project folder has a newer Soundkeep than this copy ({running_version}).")
    } else {
        format!(
            "Soundkeep {source_version} is in your project folder. This copy is {running_version}."
        )
    };
    format!(
        "{intro} Install it? Soundkeep will close, rebuild, and open again. The application menu, taskbar, and shortcuts will use the new version. Building can take a few minutes."
    )
}

// ------------------------------------------------------------
// Function: cargo_package_version
// Purpose: Reads the package version. Later version keys, such as
//          dependency versions, are ignored.
// ------------------------------------------------------------
pub fn cargo_package_version(toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some(rest) = line.strip_prefix("version") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let value = rest.trim().trim_matches('"').trim();
        if value.is_empty() {
            return None;
        }
        return Some(value.to_string());
    }
    None
}

// ------------------------------------------------------------
// Function: launch_installer
// Purpose: Opens a terminal running the upgrade installer, then
//          the caller exits so this copy does not stay running.
// ------------------------------------------------------------
pub fn launch_installer() -> io::Result<()> {
    let root = Path::new(SOURCE_ROOT);
    let script = install_script(root);
    if !script.is_file() {
        return Err(io::Error::other("installer script is missing"));
    }
    #[cfg(windows)]
    {
        launch_windows(&script)
    }
    #[cfg(not(windows))]
    {
        launch_unix(&script)
    }
}

// ------------------------------------------------------------
// Function: checkout_ready
// Purpose: The folder still looks like the Soundkeep project.
// ------------------------------------------------------------
fn checkout_ready(root: &Path) -> bool {
    let git = root.join(".git");
    (git.is_dir() || git.is_file()) && install_script(root).is_file()
}

fn install_script(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.join("scripts").join("install.ps1")
    } else {
        root.join("scripts").join("install.sh")
    }
}

fn git_head(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8(output.stdout).ok()?;
    let sha = sha.trim().to_string();
    if is_commit(&sha) { Some(sha) } else { None }
}

fn is_commit(sha: &str) -> bool {
    let sha = sha.trim();
    sha.len() >= 7 && sha.chars().all(|character| character.is_ascii_hexdigit())
}

#[cfg(windows)]
fn launch_windows(script: &Path) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-NoExit",
            "-File",
        ])
        .arg(script)
        .arg("-FromApp")
        .creation_flags(CREATE_NEW_CONSOLE | CREATE_NEW_PROCESS_GROUP)
        .spawn()?;
    Ok(())
}

#[cfg(not(windows))]
fn launch_unix(script: &Path) -> io::Result<()> {
    for program in terminal_programs() {
        if spawn_terminal(&program, script).is_ok() {
            return Ok(());
        }
    }
    spawn_logged(script)
}

#[cfg(not(windows))]
fn terminal_programs() -> Vec<String> {
    let mut programs = Vec::new();
    if let Ok(terminal) = std::env::var("TERMINAL") {
        if let Some(program) = terminal.split_whitespace().next() {
            if !program.is_empty() {
                programs.push(program.to_string());
            }
        }
    }
    for name in [
        "foot",
        "kitty",
        "alacritty",
        "ghostty",
        "gnome-terminal",
        "konsole",
        "xterm",
    ] {
        if !programs.iter().any(|program| program == name) {
            programs.push(name.to_string());
        }
    }
    programs
}

#[cfg(not(windows))]
fn spawn_terminal(program: &str, script: &Path) -> io::Result<()> {
    let mut command = Command::new(program);
    let base = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program);
    match base {
        "gnome-terminal" => {
            command.arg("--").arg("bash").arg(script).arg("--from-app");
        }
        "foot" | "kitty" => {
            command.arg("bash").arg(script).arg("--from-app");
        }
        _ => {
            command.arg("-e").arg("bash").arg(script).arg("--from-app");
        }
    }
    spawn_detached(command)
}

// ------------------------------------------------------------
// Function: spawn_detached
// Purpose: The installer keeps running after this program exits.
// ------------------------------------------------------------
#[cfg(not(windows))]
fn spawn_detached(mut command: Command) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn()?;
    Ok(())
}

#[cfg(not(windows))]
fn spawn_logged(script: &Path) -> io::Result<()> {
    let dir = config::data_dir();
    std::fs::create_dir_all(&dir)?;
    let log = std::fs::File::create(dir.join("install.log"))?;
    let error_log = log.try_clone()?;
    let mut command = Command::new("bash");
    command
        .arg(script)
        .arg("--from-app")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(error_log));
    spawn_detached(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const NEW: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[test]
    fn offer_when_the_checkout_commit_differs() {
        assert!(should_offer(OLD, NEW, "", true));
    }

    #[test]
    fn skip_when_the_commit_matches_or_was_dismissed() {
        assert!(!should_offer(OLD, OLD, "", true));
        assert!(!should_offer(OLD, NEW, NEW, true));
        assert!(!should_offer("unknown", NEW, "", true));
        assert!(!should_offer(OLD, NEW, "", false));
        assert!(!should_offer(OLD, "", "", true));
    }

    #[test]
    fn package_version_ignores_dependency_versions() {
        let toml = r#"
            [package]
            name = "soundkeep"
            version = "0.1.14"

            [dependencies]
            chrono = { version = "0.4.45" }
        "#;
        assert_eq!(cargo_package_version(toml).as_deref(), Some("0.1.14"));
    }

    #[test]
    fn prompt_names_both_versions() {
        let body = prompt_body("0.1.13", "0.1.14");
        assert!(body.contains("Soundkeep 0.1.14 is in your project folder."));
        assert!(body.contains("This copy is 0.1.13."));
        assert!(body.contains("application menu, taskbar, and shortcuts"));
    }

    #[test]
    fn prompt_when_the_version_number_did_not_change() {
        let body = prompt_body("0.1.14", "0.1.14");
        assert!(body.contains("newer Soundkeep than this copy (0.1.14)"));
    }
}
