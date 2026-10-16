// ------------------------------------------------------------
// Module: config
// Purpose: Shared constants and filesystem locations for the player.
//          Debug builds use a separate data folder and socket so they
//          never touch an installed copy's playlists or settings.
// ------------------------------------------------------------

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

/// Window title and shortcut name.
pub const APP_NAME: &str = "MP3Player";

/// Cargo.toml is the single source of truth for this string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How often the interface asks the audio thread for the playhead.
pub const TICK: std::time::Duration = std::time::Duration::from_millis(250);

/// Loudest linear gain. 1.0 is 100%, the same level as an unamplified file.
pub const MAX_VOLUME: f32 = 2.0;

/// How often a playing episode writes its position to disk.
pub const RESUME_SAVE_EVERY: std::time::Duration = std::time::Duration::from_secs(10);

/// An episode this close to its end counts as finished, not paused.
pub const RESUME_FINISHED_MARGIN: std::time::Duration = std::time::Duration::from_secs(30);

/// Extensions symphonia can decode with the `symphonia-all` feature.
pub const EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "opus", "wav", "aac", "m4a", "mp4", "alac", "aiff", "aif", "caf",
];

/// Local-socket name. Dev and installed builds must not share one instance.
pub const SOCKET_NAME: &str = if cfg!(debug_assertions) {
    "mp3player-dev.sock"
} else {
    "mp3player.sock"
};

// ------------------------------------------------------------
// Function: data_dir
// Purpose: Returns the per-user folder for settings and playlists.
// Output: Absolute path. Created later, on first save.
// ------------------------------------------------------------
pub fn data_dir() -> PathBuf {
    let folder = if cfg!(debug_assertions) {
        "MP3Player-dev"
    } else {
        "MP3Player"
    };
    ProjectDirs::from("app", "MP3Player", folder)
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join(folder))
}

// ------------------------------------------------------------
// Function: settings_path
// Purpose: JSON file that stores theme, volume, and the last queue.
// ------------------------------------------------------------
pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

// ------------------------------------------------------------
// Function: playlist_dir
// Purpose: Folder of user playlists stored as `.m3u8` files.
// ------------------------------------------------------------
pub fn playlist_dir() -> PathBuf {
    data_dir().join("playlists")
}

// ------------------------------------------------------------
// Function: podcast_dir
// Purpose: Favorites, episode cache, and downloaded audio.
// ------------------------------------------------------------
pub fn podcast_dir() -> PathBuf {
    data_dir().join("podcasts")
}

pub fn favorites_path() -> PathBuf {
    podcast_dir().join("favorites.json")
}

pub fn favorite_groups_path() -> PathBuf {
    podcast_dir().join("favorite_groups.json")
}

pub fn layout_path() -> PathBuf {
    podcast_dir().join("layout.json")
}

pub fn positions_path() -> PathBuf {
    podcast_dir().join("positions.json")
}

pub fn podcast_download_dir() -> PathBuf {
    podcast_dir().join("downloads")
}

pub fn stream_cache_dir() -> PathBuf {
    podcast_dir().join("stream-cache")
}

pub fn user_agent() -> String {
    format!("{APP_NAME}/{VERSION}")
}

// ------------------------------------------------------------
// Function: is_audio
// Purpose: True when the file extension is one we try to play.
// ------------------------------------------------------------
pub fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            EXTENSIONS
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_is_absolute_and_local_on_windows() {
        let dir = data_dir();
        assert!(dir.is_absolute());
        if cfg!(windows) {
            let local = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA");
            assert!(
                dir.starts_with(&local),
                "data dir {} should live under {}",
                dir.display(),
                local
            );
        }
    }

    #[test]
    fn extension_check_is_case_insensitive() {
        assert!(is_audio(Path::new("Song.MP3")));
        assert!(is_audio(Path::new("track.flac")));
        assert!(!is_audio(Path::new("notes.txt")));
        assert!(!is_audio(Path::new("no-extension")));
    }
}
