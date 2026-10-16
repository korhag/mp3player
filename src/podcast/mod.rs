// ------------------------------------------------------------
// Module: podcast
// Purpose: Favorite shows, episode lists, and Podcast Index search.
// ------------------------------------------------------------

mod download;
mod feed;
mod groups;
mod layout;
mod positions;
mod search;
mod signup;
mod store;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use iced::widget::pane_grid;
use serde::{Deserialize, Serialize};

pub use download::{download_task, episode_path, remove_download};
pub use feed::{fetch_artwork, fetch_show};
pub use groups::{
    DragGesture, DragItem, DropSlot, DropTarget, FavoriteItem, GroupChange, container_index,
    forget_show, group_index_of, group_with, load_groups, move_group, move_group_by, move_show,
    reconcile, remove_from_group, rename_group, save_groups, toggle_collapsed, ungroup,
};
pub use layout::{PodcastPane, default_panes, load_panes, save_panes};
pub use positions::{
    ListenState, Listening, listen_state, load_listening, mark_played, note_listen, resume_point,
    save_listening,
};
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
    /// Plain-text summary from the feed or the directory. Empty on old caches.
    #[serde(default)]
    pub description: String,
    pub episodes: Vec<Episode>,
}

// ------------------------------------------------------------
// Struct: PodcastState
// Purpose: Everything the Podcasts tab draws and edits.
// ------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct PodcastState {
    pub favorites: Vec<Show>,
    pub groups: Vec<FavoriteItem>,
    pub query: String,
    pub results: Vec<Show>,
    pub selected: Option<String>,
    pub preview: Option<Show>,
    pub key_input: String,
    pub secret_input: String,
    pub saved_key: String,
    pub saved_secret: String,
    pub key_status: KeyStatus,
    pub offline: bool,
    pub older_loaded: HashSet<String>,
    /// Download fraction, keyed by feed URL and episode guid.
    pub progress: HashMap<String, f32>,
    pub listening: Listening,
    pub panes: pane_grid::State<PodcastPane>,
    pub artwork_pending: HashSet<String>,
    pub drag: Option<DragGesture>,
    pub drop_target: Option<DropTarget>,
    pub episode_filter: String,
    pub rename_group: Option<usize>,
    pub group_rename_text: String,
    pub selected_episode: Option<(String, String)>,
    /// The detail pane can hide the show summary and keep the episode list.
    pub episodes_only: bool,
    /// Skip writing the playhead until the track changes. Set when the
    /// listener marks the current episode played or unplayed by hand.
    pub resume_skip_sync: bool,
}

impl PodcastState {
    pub fn load(key: &str, secret: &str) -> Self {
        let favorites = load_favorites();
        let groups = reconcile(load_groups(), &favorites);
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
            groups,
            query: String::new(),
            results: Vec::new(),
            selected,
            preview: None,
            key_input: stored_key.clone(),
            secret_input: stored_secret.clone(),
            saved_key: stored_key,
            saved_secret: stored_secret,
            key_status,
            offline: false,
            older_loaded: HashSet::new(),
            progress: HashMap::new(),
            listening: load_listening(),
            panes: load_panes(),
            artwork_pending: HashSet::new(),
            drag: None,
            drop_target: None,
            episode_filter: String::new(),
            rename_group: None,
            group_rename_text: String::new(),
            selected_episode: None,
            episodes_only: false,
            resume_skip_sync: false,
        }
    }

    pub fn selected_show(&self) -> Option<&Show> {
        let feed = self.selected.as_deref()?;
        self.find_show(feed)
    }

    // --------------------------------------------------------
    // Method: find_show
    // Purpose: A favorite wins over the search preview when both
    //          exist for the same feed.
    // --------------------------------------------------------
    pub fn find_show(&self, feed_url: &str) -> Option<&Show> {
        self.favorites
            .iter()
            .find(|show| show.feed_url == feed_url)
            .or_else(|| {
                self.preview
                    .as_ref()
                    .filter(|show| show.feed_url == feed_url)
            })
    }

    pub fn is_favorite(&self, feed_url: &str) -> bool {
        self.favorites.iter().any(|show| show.feed_url == feed_url)
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
// Function: feed_url
// Purpose: The search box treats an http(s) URL as a feed to
//          preview and anything else as a directory search.
// ------------------------------------------------------------
pub fn feed_url(text: &str) -> Option<String> {
    let text = text.trim();
    let url = reqwest::Url::parse(text).ok()?;
    if matches!(url.scheme(), "http" | "https") {
        Some(text.to_string())
    } else {
        None
    }
}

// ------------------------------------------------------------
// Function: plain_text
// Purpose: Collapses a feed summary to a single readable line.
// ------------------------------------------------------------
pub fn plain_text(value: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(character),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
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

// ------------------------------------------------------------
// Function: apply_learned_duration
// Purpose: Fills a missing episode length from the decoder.
//          A length the feed already gave us is left alone.
// ------------------------------------------------------------
pub fn apply_learned_duration(current_secs: u64, learned: Option<Duration>) -> Option<u64> {
    if current_secs > 0 {
        return None;
    }
    let learned = learned?;
    let secs = learned.as_secs();
    (secs > 0).then_some(secs)
}

// ------------------------------------------------------------
// Function: store_learned_duration
// Purpose: Writes a decoder length onto cached episodes that
//          share this audio URL and still have no length.
//          Returns whether a favorite needs saving.
// ------------------------------------------------------------
pub fn store_learned_duration(
    shows: &mut [Show],
    audio_url: &str,
    learned: Option<Duration>,
) -> bool {
    if audio_url.is_empty() {
        return false;
    }
    let mut changed = false;
    for show in shows {
        for episode in &mut show.episodes {
            if episode.audio_url != audio_url {
                continue;
            }
            let Some(secs) = apply_learned_duration(episode.duration_secs, learned) else {
                continue;
            };
            episode.duration_secs = secs;
            changed = true;
        }
    }
    changed
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

    #[test]
    fn feed_url_accepts_http_and_rejects_a_search_term() {
        assert_eq!(
            feed_url("  https://example.com/feed.xml "),
            Some("https://example.com/feed.xml".to_string())
        );
        assert_eq!(
            feed_url("http://example.com/rss"),
            Some("http://example.com/rss".to_string())
        );
        assert_eq!(feed_url("part of the problem"), None);
        assert_eq!(feed_url("ftp://example.com/feed.xml"), None);
    }

    #[test]
    fn learned_duration_fills_only_a_missing_length() {
        let learned = Some(Duration::from_secs(3723));
        assert_eq!(apply_learned_duration(0, learned), Some(3723));
        assert_eq!(apply_learned_duration(90, learned), None);
        assert_eq!(apply_learned_duration(0, None), None);
        assert_eq!(apply_learned_duration(0, Some(Duration::ZERO)), None);
        assert_eq!(
            apply_learned_duration(0, Some(Duration::from_millis(400))),
            None
        );

        let known = episode("kept", "https://example.com/kept.mp3", None);
        let mut missing = episode("missing", "https://example.com/missing.mp3", None);
        missing.duration_secs = 0;
        let mut shows = vec![Show {
            feed_url: "https://example.com/feed.xml".to_string(),
            title: "Sample".to_string(),
            author: String::new(),
            image_url: None,
            description: String::new(),
            episodes: vec![known.clone(), missing],
        }];
        assert!(store_learned_duration(
            &mut shows,
            "https://example.com/missing.mp3",
            learned
        ));
        assert_eq!(shows[0].episodes[0], known);
        assert_eq!(shows[0].episodes[1].duration_secs, 3723);
        assert!(!store_learned_duration(
            &mut shows,
            "https://example.com/missing.mp3",
            learned
        ));
    }
}
