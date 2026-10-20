// ------------------------------------------------------------
// Module: playlist
// Purpose: Saves and loads playlists as M3U8 so other players can
//          open the same files.
// ------------------------------------------------------------

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::track::Track;

// ------------------------------------------------------------
// Struct: PlaylistEntry
// Purpose: A playlist name shown in the sidebar. The file is
//          `<name>.m3u8` inside the playlist directory.
// ------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistEntry {
    pub name: String,
}

// ------------------------------------------------------------
// Function: list
// Purpose: Returns saved playlists in case-insensitive name order.
// ------------------------------------------------------------
pub fn list(dir: &Path) -> io::Result<Vec<PlaylistEntry>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let is_playlist = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("m3u8"));
        if !is_playlist {
            continue;
        }
        if let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) {
            entries.push(PlaylistEntry {
                name: name.to_string(),
            });
        }
    }
    entries.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(entries)
}

// ------------------------------------------------------------
// Function: save
// Purpose: Writes the current queue as an M3U8 playlist.
// Input: dir - Playlist folder. name - Display name.
//        tracks - Queue in display order.
// Output: Path of the written file.
// ------------------------------------------------------------
pub fn save(dir: &Path, name: &str, tracks: &[Track]) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = playlist_path(dir, name);
    let mut file = fs::File::create(&path)?;
    writeln!(file, "#EXTM3U")?;
    for track in tracks {
        let seconds = i64::try_from(track.duration.as_secs()).unwrap_or(-1);
        let artist = if track.artist.is_empty() {
            "Unknown artist"
        } else {
            track.artist.as_str()
        };
        writeln!(file, "#EXTINF:{seconds},{artist} - {}", track.title)?;
        let text = track.path.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "track path is not Unicode")
        })?;
        writeln!(file, "{text}")?;
    }
    Ok(path)
}

// ------------------------------------------------------------
// Function: load
// Purpose: Reads audio paths from an M3U8 playlist.
//          Relative paths are resolved next to the playlist file.
// ------------------------------------------------------------
pub fn load(dir: &Path, name: &str) -> io::Result<Vec<PathBuf>> {
    let path = playlist_path(dir, name);
    let text = fs::read_to_string(&path)?;
    let parent = path.parent().unwrap_or(dir);
    let mut paths = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let track_path = PathBuf::from(line);
        let track_path = if track_path.is_absolute() {
            track_path
        } else {
            parent.join(track_path)
        };
        paths.push(track_path);
    }
    Ok(paths)
}

// ------------------------------------------------------------
// Function: rename
// Purpose: Renames a playlist file. Fails if the new name exists.
// ------------------------------------------------------------
pub fn rename(dir: &Path, from: &str, to: &str) -> io::Result<()> {
    let source = playlist_path(dir, from);
    let target = playlist_path(dir, to);
    if target.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "a playlist with that name already exists",
        ));
    }
    fs::rename(source, target)
}

// ------------------------------------------------------------
// Function: delete
// Purpose: Removes a playlist file. Does not delete audio files.
// ------------------------------------------------------------
pub fn delete(dir: &Path, name: &str) -> io::Result<()> {
    fs::remove_file(playlist_path(dir, name))
}

// ------------------------------------------------------------
// Function: replace_path
// Purpose: Rewrites saved playlists that still point at a file
//          which the user has relocated. Other lines stay put.
// Output: How many playlist files changed.
// ------------------------------------------------------------
pub fn replace_path(dir: &Path, old: &Path, new: &Path) -> io::Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut changed_files = 0;
    for entry in list(dir)? {
        let file = playlist_path(dir, &entry.name);
        let text = fs::read_to_string(&file)?;
        let mut changed = false;
        let mut lines = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                lines.push(line.to_string());
                continue;
            }
            let track_path = PathBuf::from(trimmed);
            if track_path == old {
                let text = new.to_str().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "new path is not Unicode")
                })?;
                lines.push(text.to_string());
                changed = true;
            } else {
                lines.push(line.to_string());
            }
        }
        if changed {
            lines.push(String::new());
            fs::write(&file, lines.join("\n"))?;
            changed_files += 1;
        }
    }
    Ok(changed_files)
}

// ------------------------------------------------------------
// Function: playlist_path
// Purpose: Maps a display name to a safe `.m3u8` filename.
// ------------------------------------------------------------
pub fn playlist_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{}.m3u8", sanitize_name(name)))
}

// ------------------------------------------------------------
// Function: sanitize_name
// Purpose: Strips characters that are illegal in file names.
// ------------------------------------------------------------
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "playlist".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("soundkeep-playlist-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn save_load_rename_and_delete_round_trip() {
        let dir = temp_dir();
        let tracks = vec![
            Track::from_path(Path::new("C:/Music/one.mp3")),
            Track::from_path(Path::new("C:/Music/two.flac")),
        ];
        save(&dir, "Evening", &tracks).unwrap();
        let loaded = load(&dir, "Evening").unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded[0].ends_with("one.mp3"));
        assert!(loaded[1].ends_with("two.flac"));

        let text = fs::read_to_string(playlist_path(&dir, "Evening")).unwrap();
        assert!(text.starts_with("#EXTM3U"));

        rename(&dir, "Evening", "Morning").unwrap();
        assert!(load(&dir, "Morning").is_ok());
        assert!(load(&dir, "Evening").is_err());

        let names: Vec<_> = list(&dir)
            .unwrap()
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(names, vec!["Morning".to_string()]);

        delete(&dir, "Morning").unwrap();
        assert!(list(&dir).unwrap().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_path_rewrites_only_matching_playlists() {
        let dir = temp_dir();
        let old = dir.join("old.mp3");
        let new = dir.join("new.mp3");
        let other = dir.join("other.flac");
        fs::write(&old, b"").unwrap();
        fs::write(&new, b"").unwrap();
        fs::write(&other, b"").unwrap();
        save(
            &dir,
            "Moved",
            &[Track::from_path(&old), Track::from_path(&other)],
        )
        .unwrap();
        save(&dir, "Untouched", &[Track::from_path(&other)]).unwrap();

        assert_eq!(replace_path(&dir, &old, &new).unwrap(), 1);
        let moved = load(&dir, "Moved").unwrap();
        assert_eq!(moved[0], new);
        assert_eq!(moved[1], other);
        assert_eq!(load(&dir, "Untouched").unwrap(), vec![other]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sanitize_replaces_path_separators() {
        assert_eq!(sanitize_name(" a/b:c "), "a_b_c");
        assert_eq!(sanitize_name("..."), "playlist");
    }
}
