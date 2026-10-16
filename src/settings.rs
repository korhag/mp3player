// ------------------------------------------------------------
// Module: settings
// Purpose: Persists theme, volume, shuffle, repeat, and the last
//          queue as JSON in the per-user data folder.
// ------------------------------------------------------------

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config;
use crate::queue::RepeatMode;
use crate::theme::ThemeId;

fn default_volume() -> f32 {
    0.8
}

// ------------------------------------------------------------
// Struct: Settings
// Purpose: Everything the player restores on the next launch.
// ------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: ThemeId,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub shuffle: bool,
    #[serde(default)]
    pub repeat: RepeatMode,
    #[serde(default)]
    pub queue: Vec<PathBuf>,
    #[serde(default)]
    pub queue_index: Option<usize>,
    #[serde(default)]
    pub podcast_index_key: String,
    #[serde(default)]
    pub podcast_index_secret: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeId::MatteBlack,
            volume: default_volume(),
            shuffle: false,
            repeat: RepeatMode::Off,
            queue: Vec::new(),
            queue_index: None,
            podcast_index_key: String::new(),
            podcast_index_secret: String::new(),
        }
    }
}

impl Settings {
    // --------------------------------------------------------
    // Function: load
    // Purpose: Reads the installed (or dev) settings file.
    //          Missing or unreadable files return defaults.
    // --------------------------------------------------------
    pub fn load() -> Self {
        Self::load_from(&config::settings_path())
    }

    // --------------------------------------------------------
    // Function: load_from
    // Purpose: Reads settings from an explicit path. Used by tests.
    // --------------------------------------------------------
    pub fn load_from(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str::<Settings>(&text) {
            Ok(mut settings) => {
                if !settings.volume.is_finite() {
                    settings.volume = default_volume();
                }
                settings.volume = settings.volume.clamp(0.0, config::MAX_VOLUME);
                settings
            }
            Err(_) => Self::default(),
        }
    }

    // --------------------------------------------------------
    // Function: save
    // Purpose: Writes settings to the per-user data folder.
    // --------------------------------------------------------
    pub fn save(&self) -> io::Result<()> {
        self.save_to(&config::settings_path())
    }

    // --------------------------------------------------------
    // Function: save_to
    // Purpose: Writes settings to an explicit path.
    // --------------------------------------------------------
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "mp3player-settings-{}-{nanos}.json",
            std::process::id()
        ))
    }

    #[test]
    fn missing_file_uses_defaults() {
        let settings = Settings::load_from(Path::new("this-file-does-not-exist.json"));
        assert_eq!(settings.theme, ThemeId::MatteBlack);
        assert!((settings.volume - 0.8).abs() < f32::EPSILON);
        assert!(!settings.shuffle);
        assert_eq!(settings.repeat, RepeatMode::Off);
        assert!(settings.queue.is_empty());
    }

    #[test]
    fn round_trip_and_corrupt_file() {
        let path = temp_file();
        let mut settings = Settings::default();
        settings.theme = ThemeId::OsakaJade;
        settings.volume = 0.4;
        settings.shuffle = true;
        settings.repeat = RepeatMode::One;
        settings.queue = vec![PathBuf::from("C:/Music/song.mp3")];
        settings.queue_index = Some(0);
        settings.save_to(&path).unwrap();

        let loaded = Settings::load_from(&path);
        assert_eq!(loaded.theme, ThemeId::OsakaJade);
        assert!((loaded.volume - 0.4).abs() < f32::EPSILON);
        assert!(loaded.shuffle);
        assert_eq!(loaded.repeat, RepeatMode::One);
        assert_eq!(loaded.queue.len(), 1);
        assert_eq!(loaded.queue_index, Some(0));

        fs::write(&path, "{not json").unwrap();
        let fallback = Settings::load_from(&path);
        assert_eq!(fallback.theme, ThemeId::MatteBlack);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn volume_above_full_scale_round_trips_and_oversized_values_clamp() {
        let path = temp_file();
        let mut settings = Settings::default();
        settings.volume = 1.5;
        settings.save_to(&path).unwrap();
        let loaded = Settings::load_from(&path);
        assert!((loaded.volume - 1.5).abs() < f32::EPSILON);

        let mut loud = Settings::default();
        loud.volume = 5.0;
        loud.save_to(&path).unwrap();
        let clamped = Settings::load_from(&path);
        assert!((clamped.volume - config::MAX_VOLUME).abs() < f32::EPSILON);
        let _ = fs::remove_file(path);
    }
}
