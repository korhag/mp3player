// ------------------------------------------------------------
// Module: library
// Purpose: Track metadata, playlist files, and folder scanning.
// ------------------------------------------------------------

mod playlist;
mod track;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub use playlist::{PlaylistEntry, delete, list, load, rename, replace_path, sanitize_name, save};
pub use track::Track;

use crate::config;

// ------------------------------------------------------------
// Function: collect_audio_paths
// Purpose: Expands files and folders into supported audio paths.
//          Folder contents are sorted; the caller's file order
//          is kept. Symlinks are skipped so a link cannot loop.
// ------------------------------------------------------------
pub fn collect_audio_paths(inputs: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    for input in inputs {
        if input.is_dir() {
            walk_dir(input, &mut found, &mut seen, 0);
        } else if config::is_audio(input) && seen.insert(input.clone()) {
            found.push(input.clone());
        }
    }
    found
}

fn walk_dir(dir: &Path, found: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, depth: u32) {
    if depth > 32 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            dirs.push(entry.path());
        } else if config::is_audio(&entry.path()) {
            files.push(entry.path());
        }
    }
    files.sort();
    dirs.sort();
    for file in files {
        if seen.insert(file.clone()) {
            found.push(file);
        }
    }
    for dir in dirs {
        walk_dir(&dir, found, seen, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_folders_and_ignores_other_files() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("soundkeep-scan-{}-{nanos}", std::process::id()));
        let nested = root.join("album");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("notes.txt"), b"hello").unwrap();
        fs::write(root.join("root.mp3"), b"").unwrap();
        fs::write(nested.join("b.flac"), b"").unwrap();
        fs::write(nested.join("a.ogg"), b"").unwrap();

        let found = collect_audio_paths(&[root.clone(), root.join("root.mp3")]);
        let names: Vec<_> = found
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["root.mp3", "a.ogg", "b.flac"]);
        let _ = fs::remove_dir_all(root);
    }
}
