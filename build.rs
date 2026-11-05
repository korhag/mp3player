// ------------------------------------------------------------
// Build script
// Purpose: Embeds the application icon and version resource into the
//          Windows executable, and records the git commit this binary
//          was built from. Cargo.toml is the version source.
// ------------------------------------------------------------

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    watch_git_head(&manifest_dir);
    println!(
        "cargo:rustc-env=SOUNDKEEP_GIT_SHA={}",
        git_sha(&manifest_dir)
    );

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource
            .compile()
            .expect("embed Windows icon and version resource");
    }
}

// ------------------------------------------------------------
// Function: watch_git_head
// Purpose: Rebuilds when the current branch moves, so a later
//          compile embeds the new commit.
// ------------------------------------------------------------
fn watch_git_head(manifest_dir: &Path) {
    let git_dir = git_dir(manifest_dir);
    let head_path = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head_path.display());
    let Ok(head) = std::fs::read_to_string(&head_path) else {
        return;
    };
    let Some(reference) = head.trim().strip_prefix("ref: ") else {
        return;
    };
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join(reference.trim()).display()
    );
}

// ------------------------------------------------------------
// Function: git_dir
// Purpose: Resolves .git, including a worktree gitdir pointer.
// ------------------------------------------------------------
fn git_dir(manifest_dir: &Path) -> PathBuf {
    let git = manifest_dir.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(dir) = text.trim().strip_prefix("gitdir: ") {
                let dir = PathBuf::from(dir.trim());
                if dir.is_absolute() {
                    return dir;
                }
                return manifest_dir.join(dir);
            }
        }
    }
    git
}

// ------------------------------------------------------------
// Function: git_sha
// Purpose: Full commit this build came from. "unknown" skips the
//          update prompt when git is not available.
// ------------------------------------------------------------
fn git_sha(manifest_dir: &Path) -> String {
    let Ok(output) = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(manifest_dir)
        .output()
    else {
        return "unknown".to_string();
    };
    if !output.status.success() {
        return "unknown".to_string();
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        "unknown".to_string()
    } else {
        sha
    }
}
