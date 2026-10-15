// ------------------------------------------------------------
// Module: podcast
// Purpose: Favorite shows, episode lists, and Podcast Index search.
// ------------------------------------------------------------

mod download;
mod feed;
mod search;
mod signup;
mod store;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub use download::{download_task, episode_path, remove_download};
pub use feed::fetch_show;
pub use search::{check_key, credentials, older_episodes, search, using_environment};
pub use signup::{SignupPlace, open_signup};
pub use store::{load_favorites, save_favorites};

// ------------------------------------------------------------
// Struct: Episode
// Purpose: One episode cached with its show. `local_path` is set
//          after a finished download.
// ------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Episode {
    pub guid: String,
    pub title: String,
    pub audio_url: String,
    pub duration_secs: u64,
    /// Unix seconds from the feed's pubDate. Missing on older caches.
    #[serde(default)]
    pub published_secs: Option<i64>,
    pub local_path: Option<PathBuf>,
}

// ------------------------------------------------------------
// Enum: KeyStatus
// Purpose: Whether the Podcast Index key is saved and accepted.
// ------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStatus {
    Missing,
    Unchecked,
    Checking,
    Valid,
    Rejected,
    Unreachable(String),
}

impl Episode {
    pub fn duration(&self) -> Duration {
        Duration::from_secs(self.duration_secs)
    }

    pub fn is_downloaded(&self) -> bool {
        self.local_path.as_ref().is_some_and(|path| path.is_file())
    }
}

// ------------------------------------------------------------
// Struct: Show
// Purpose: A favorite or a search hit. Search hits have no episodes
//          until the RSS feed is read.
// ------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Show {
    pub feed_url: String,
    pub title: String,
    pub author: String,
    pub image_url: Option<String>,
    pub episodes: Vec<Episode>,
}

// ------------------------------------------------------------
// Struct: PodcastState
// Purpose: Everything the Podcasts tab draws and edits.
// ------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct PodcastState {
    pub favorites: Vec<Show>,
    pub query: String,
    pub results: Vec<Show>,
    pub rss_input: String,
    pub selected: Option<String>,
    pub key_input: String,
    pub secret_input: String,
    pub saved_key: String,
    pub saved_secret: String,
    pub key_status: KeyStatus,
    pub offline: bool,
    pub older_loaded: HashSet<String>,
    pub progress: HashMap<String, f32>,
}

impl PodcastState {
    pub fn load(key: &str, secret: &str) -> Self {
        let favorites = load_favorites();
        let selected = favorites.first().map(|show| show.feed_url.clone());
        let stored_key = key.trim().to_string();
        let stored_secret = secret.trim().to_string();
        let key_status = if credentials(&stored_key, &stored_secret).is_some() {
            KeyStatus::Unchecked
        } else {
            KeyStatus::Missing
        };
        Self {
            favorites,
            query: String::new(),
            results: Vec::new(),
            rss_input: String::new(),
            selected,
            key_input: stored_key.clone(),
            secret_input: stored_secret.clone(),
            saved_key: stored_key,
            saved_secret: stored_secret,
            key_status,
            offline: false,
            older_loaded: HashSet::new(),
            progress: HashMap::new(),
        }
    }

    pub fn selected_show(&self) -> Option<&Show> {
        let feed = self.selected.as_deref()?;
        self.favorites.iter().find(|show| show.feed_url == feed)
    }

    pub fn favorite_mut(&mut self, feed_url: &str) -> Option<&mut Show> {
        self.favorites
            .iter_mut()
            .find(|show| show.feed_url == feed_url)
    }

    pub fn progress_key(feed_url: &str, guid: &str) -> String {
        format!("{feed_url}\n{guid}")
    }
}

// ------------------------------------------------------------
// Function: merge_episodes
// Purpose: Combines a fresh list with episodes already cached.
//          `primary` wins the title. Download paths and dates are
//          kept from whichever side has them. Episodes that only
//          exist in `extra` stay, so a feed refresh does not drop
//          older history. The result is newest first.
// ------------------------------------------------------------
pub fn merge_episodes(primary: &[Episode], extra: &[Episode]) -> Vec<Episode> {
    let mut merged = primary.to_vec();
    for episode in extra {
        if let Some(existing) = merged.iter_mut().find(|item| same_episode(item, episode)) {
            if existing.local_path.is_none() {
                existing.local_path = episode.local_path.clone();
            }
            if existing.published_secs.is_none() {
                existing.published_secs = episode.published_secs;
            }
            if existing.duration_secs == 0 {
                existing.duration_secs = episode.duration_secs;
            }
        } else {
            merged.push(episode.clone());
        }
    }
    merged.sort_by(|left, right| {
        right
            .published_secs
            .unwrap_or(i64::MIN)
            .cmp(&left.published_secs.unwrap_or(i64::MIN))
    });
    merged
}

fn same_episode(left: &Episode, right: &Episode) -> bool {
    if !left.guid.is_empty() && left.guid == right.guid {
        return true;
    }
    !left.audio_url.is_empty() && left.audio_url == right.audio_url
}

#[cfg(test)]
mod tests {
    use super::*;

    fn episode(guid: &str, audio_url: &str, published: Option<i64>) -> Episode {
        Episode {
            guid: guid.to_string(),
            title: guid.to_string(),
            audio_url: audio_url.to_string(),
            duration_secs: 10,
            published_secs: published,
            local_path: None,
        }
    }

    #[test]
    fn merge_keeps_downloads_history_and_sorts_newest_first() {
        let mut current = episode("new", "https://example.com/new.mp3", Some(200));
        current.local_path = Some(PathBuf::from("C:/podcasts/new.mp3"));
        current.published_secs = None;
        let dated = episode("new", "https://example.com/new.mp3", Some(200));
        let older = episode("old", "https://example.com/old.mp3", Some(100));
        let duplicate = episode("old", "https://example.com/old.mp3", Some(100));

        let merged = merge_episodes(&[current], &[dated, older, duplicate]);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].guid, "new");
        assert_eq!(merged[0].published_secs, Some(200));
        assert_eq!(
            merged[0].local_path.as_deref(),
            Some(std::path::Path::new("C:/podcasts/new.mp3"))
        );
        assert_eq!(merged[1].guid, "old");
        assert_eq!(merged[1].published_secs, Some(100));
    }
}
