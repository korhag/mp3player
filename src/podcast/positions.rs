// ------------------------------------------------------------
// Module: positions
// Purpose: Remembers how far the listener got in each episode
//          and which episodes they finished.
// ------------------------------------------------------------

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::config;

// ------------------------------------------------------------
// Struct: Listening
// Purpose: Positions are seconds keyed by the episode audio URL.
//          `played` holds URLs that finished or were marked played.
// ------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Listening {
    #[serde(default)]
    pub positions: HashMap<String, f32>,
    #[serde(default)]
    pub played: HashSet<String>,
}

pub fn load_listening() -> Listening {
    load_from(&config::positions_path())
}

pub fn save_listening(listening: &Listening) -> std::io::Result<()> {
    save_to(&config::positions_path(), listening)
}

fn load_from(path: &Path) -> Listening {
    let Ok(text) = fs::read_to_string(path) else {
        return Listening::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_to(path: &Path, listening: &Listening) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(listening).map_err(std::io::Error::other)?;
    fs::write(path, json)
}

// ------------------------------------------------------------
// Function: resume_point
// Purpose: Where to start an episode that already has a saved
//          position. None means start at the beginning: the
//          saved spot is too early, or the episode was nearly
//          finished.
// ------------------------------------------------------------
pub fn resume_point(saved_secs: f32, duration: Duration) -> Option<Duration> {
    if !saved_secs.is_finite() || saved_secs < 1.0 {
        return None;
    }
    if duration > Duration::ZERO {
        let remaining = duration.as_secs_f32() - saved_secs;
        if remaining <= config::RESUME_FINISHED_MARGIN.as_secs_f32() {
            return None;
        }
    }
    Some(Duration::from_secs_f32(saved_secs))
}

// ------------------------------------------------------------
// Function: note_listen
// Purpose: Records the playhead. Near the end the episode is
//          marked played and the position is dropped.
// ------------------------------------------------------------
pub fn note_listen(listening: &mut Listening, url: &str, position: Duration, total: Duration) {
    if url.is_empty() {
        return;
    }
    let secs = position.as_secs_f32();
    if !secs.is_finite() {
        return;
    }
    let finished = total > Duration::ZERO
        && secs >= 1.0
        && total.as_secs_f32() - secs <= config::RESUME_FINISHED_MARGIN.as_secs_f32();
    if finished {
        listening.positions.remove(url);
        listening.played.insert(url.to_string());
        return;
    }
    if secs < 1.0 {
        listening.positions.remove(url);
        return;
    }
    listening.positions.insert(url.to_string(), secs);
    listening.played.remove(url);
}

// ------------------------------------------------------------
// Enum: ListenState
// Purpose: How far the listener has got, for the episode mark.
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenState {
    New,
    Started,
    Finished,
}

// ------------------------------------------------------------
// Function: listen_state
// Purpose: Finished wins over a leftover position. A saved
//          position means the episode was started.
// ------------------------------------------------------------
pub fn listen_state(listening: &Listening, url: &str) -> ListenState {
    if url.is_empty() {
        return ListenState::New;
    }
    if listening.played.contains(url) {
        return ListenState::Finished;
    }
    if listening.positions.contains_key(url) {
        return ListenState::Started;
    }
    ListenState::New
}

pub fn mark_played(listening: &mut Listening, url: &str, played: bool) {
    if url.is_empty() {
        return;
    }
    if played {
        listening.positions.remove(url);
        listening.played.insert(url.to_string());
    } else {
        listening.played.remove(url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_point_skips_the_start_and_the_ending() {
        let hour = Duration::from_secs(3600);
        assert_eq!(resume_point(0.4, hour), None);
        assert_eq!(
            resume_point(100.0, hour),
            Some(Duration::from_secs_f32(100.0))
        );
        assert_eq!(resume_point(3580.0, hour), None);
        assert_eq!(
            resume_point(50.0, Duration::ZERO),
            Some(Duration::from_secs_f32(50.0))
        );
    }

    #[test]
    fn note_listen_marks_a_nearly_finished_episode() {
        let mut listening = Listening::default();
        note_listen(
            &mut listening,
            "https://example.com/ep.mp3",
            Duration::from_secs(100),
            Duration::from_secs(3600),
        );
        assert_eq!(
            listening.positions.get("https://example.com/ep.mp3"),
            Some(&100.0)
        );
        assert_eq!(
            listen_state(&listening, "https://example.com/ep.mp3"),
            ListenState::Started
        );
        note_listen(
            &mut listening,
            "https://example.com/ep.mp3",
            Duration::from_secs(3580),
            Duration::from_secs(3600),
        );
        assert!(listening.positions.is_empty());
        assert!(listening.played.contains("https://example.com/ep.mp3"));
        assert_eq!(
            listen_state(&listening, "https://example.com/ep.mp3"),
            ListenState::Finished
        );
        assert_eq!(
            listen_state(&Listening::default(), "https://example.com/ep.mp3"),
            ListenState::New
        );
    }

    #[test]
    fn positions_round_trip() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "mp3player-positions-{}-{nanos}.json",
            std::process::id()
        ));
        let mut listening = Listening::default();
        listening
            .positions
            .insert("https://example.com/ep.mp3".to_string(), 12.5);
        listening
            .played
            .insert("https://example.com/done.mp3".to_string());
        save_to(&path, &listening).unwrap();
        assert_eq!(load_from(&path), listening);
        let _ = fs::remove_file(path);
    }
}
