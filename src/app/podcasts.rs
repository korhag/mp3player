// ------------------------------------------------------------
// Module: podcasts
// Purpose: Search, favorites, streaming playback, and downloads
//          for the Podcasts tab.
// ------------------------------------------------------------

use std::path::PathBuf;
use std::time::Duration;

use iced::Task;

use crate::library::Track;
use crate::podcast::{self, Episode, KeyStatus, Show};

use super::Message;
use super::State;

#[derive(Debug, Clone)]
pub enum PodcastMessage {
    QueryChanged(String),
    Search,
    SearchFinished(Result<Vec<Show>, String>),
    RssChanged(String),
    AddRss,
    FeedFinished {
        result: Result<Show, String>,
    },
    OpenShow(String),
    RefreshShow(String),
    RemoveShow(String),
    KeyChanged(String),
    SecretChanged(String),
    SaveKeys,
    CheckKey,
    KeyChecked(KeyStatus),
    OpenSignup,
    LoadOlder(String),
    OlderLoaded {
        feed_url: String,
        result: Result<Vec<Episode>, String>,
    },
    PlayEpisode {
        feed_url: String,
        guid: String,
    },
    QueueEpisodeNext {
        feed_url: String,
        guid: String,
    },
    DownloadEpisode {
        feed_url: String,
        guid: String,
    },
    DownloadProgress {
        key: String,
        fraction: f32,
    },
    DownloadFinished {
        feed_url: String,
        guid: String,
        result: Result<PathBuf, String>,
    },
    RemoveDownload {
        feed_url: String,
        guid: String,
    },
}

impl State {
    pub(crate) fn podcast(&mut self, message: PodcastMessage) -> Task<Message> {
        match message {
            PodcastMessage::QueryChanged(query) => {
                self.podcasts.query = query;
                Task::none()
            }
            PodcastMessage::Search => self.search_shows(),
            PodcastMessage::SearchFinished(result) => {
                match result {
                    Ok(shows) => {
                        self.podcasts.offline = false;
                        self.podcasts.key_status = KeyStatus::Valid;
                        self.status = if shows.is_empty() {
                            "No shows matched that search.".to_string()
                        } else {
                            format!("Found {} shows.", shows.len())
                        };
                        self.podcasts.results = shows;
                    }
                    Err(error) => {
                        if error.contains("rejected") {
                            self.podcasts.key_status = KeyStatus::Rejected;
                        }
                        self.status = error;
                    }
                }
                Task::none()
            }
            PodcastMessage::RssChanged(url) => {
                self.podcasts.rss_input = url;
                Task::none()
            }
            PodcastMessage::AddRss => {
                let url = self.podcasts.rss_input.trim().to_string();
                if url.is_empty() {
                    self.status = "Paste an RSS feed URL first.".to_string();
                    return Task::none();
                }
                self.status = "Reading the feed...".to_string();
                fetch_feed(url)
            }
            PodcastMessage::FeedFinished { result } => self.store_feed(result),
            PodcastMessage::OpenShow(feed_url) => {
                self.menu = None;
                self.podcasts.selected = Some(feed_url);
                Task::none()
            }
            PodcastMessage::RefreshShow(feed_url) => {
                self.menu = None;
                self.status = "Reading the feed...".to_string();
                fetch_feed(feed_url)
            }
            PodcastMessage::RemoveShow(feed_url) => {
                self.menu = None;
                self.podcasts
                    .favorites
                    .retain(|show| show.feed_url != feed_url);
                if self.podcasts.selected.as_deref() == Some(feed_url.as_str()) {
                    self.podcasts.selected = self
                        .podcasts
                        .favorites
                        .first()
                        .map(|show| show.feed_url.clone());
                }
                self.remember_favorites();
                self.status = "Removed the show from favorites.".to_string();
                Task::none()
            }
            PodcastMessage::KeyChanged(key) => {
                self.podcasts.key_input = key;
                Task::none()
            }
            PodcastMessage::SecretChanged(secret) => {
                self.podcasts.secret_input = secret;
                Task::none()
            }
            PodcastMessage::SaveKeys => self.save_keys(),
            PodcastMessage::CheckKey => self.start_key_check(),
            PodcastMessage::KeyChecked(status) => {
                if let KeyStatus::Unreachable(message) = &status {
                    self.status = message.clone();
                } else if matches!(status, KeyStatus::Rejected) {
                    self.status = "Podcast Index rejected that API key.".to_string();
                }
                self.podcasts.key_status = status;
                Task::none()
            }
            PodcastMessage::OpenSignup => {
                self.status = match podcast::open_signup() {
                    Ok(podcast::SignupPlace::Window) => {
                        "Sign up in the window that opened. Podcast Index emails the key and secret. Paste them here. Use an address that is not a free inbox such as Gmail.".to_string()
                    }
                    Ok(podcast::SignupPlace::Browser) => {
                        "Opened the Podcast Index signup page in your browser. They email the key and secret. Paste them here.".to_string()
                    }
                    Err(error) => error,
                };
                Task::none()
            }
            PodcastMessage::PlayEpisode { feed_url, guid } => {
                self.menu = None;
                self.play_episode(&feed_url, &guid, false)
            }
            PodcastMessage::QueueEpisodeNext { feed_url, guid } => {
                self.menu = None;
                self.play_episode(&feed_url, &guid, true)
            }
            PodcastMessage::DownloadEpisode { feed_url, guid } => {
                self.menu = None;
                self.start_download(&feed_url, &guid)
            }
            PodcastMessage::DownloadProgress { key, fraction } => {
                self.podcasts.progress.insert(key, fraction);
                Task::none()
            }
            PodcastMessage::DownloadFinished {
                feed_url,
                guid,
                result,
            } => self.finish_download(feed_url, guid, result),
            PodcastMessage::RemoveDownload { feed_url, guid } => {
                self.menu = None;
                self.remove_download(&feed_url, &guid);
                Task::none()
            }
            PodcastMessage::LoadOlder(feed_url) => self.load_older(feed_url),
            PodcastMessage::OlderLoaded { feed_url, result } => self.store_older(feed_url, result),
        }
    }

    fn save_keys(&mut self) -> Task<Message> {
        self.podcasts.key_input = self.podcasts.key_input.trim().to_string();
        self.podcasts.secret_input = self.podcasts.secret_input.trim().to_string();
        self.podcasts.saved_key = self.podcasts.key_input.clone();
        self.podcasts.saved_secret = self.podcasts.secret_input.clone();
        self.persist();
        let stored = !self.podcasts.saved_key.is_empty() && !self.podcasts.saved_secret.is_empty();
        if stored {
            self.status = "Saved the Podcast Index key.".to_string();
        } else if podcast::using_environment() {
            self.status = "Cleared the saved key. The environment key is still in use.".to_string();
        } else {
            self.podcasts.key_status = KeyStatus::Missing;
            self.status = "Removed the Podcast Index key.".to_string();
            return Task::none();
        }
        Task::done(Message::Podcast(PodcastMessage::CheckKey))
    }

    fn start_key_check(&mut self) -> Task<Message> {
        let Some((key, secret)) =
            podcast::credentials(&self.podcasts.saved_key, &self.podcasts.saved_secret)
        else {
            self.podcasts.key_status = KeyStatus::Missing;
            return Task::none();
        };
        self.podcasts.key_status = KeyStatus::Checking;
        Task::perform(podcast::check_key(key, secret), |status| {
            Message::Podcast(PodcastMessage::KeyChecked(status))
        })
    }

    fn load_older(&mut self, feed_url: String) -> Task<Message> {
        let Some((key, secret)) =
            podcast::credentials(&self.podcasts.saved_key, &self.podcasts.saved_secret)
        else {
            self.tab = super::Tab::Settings;
            self.status = "Add a Podcast Index API key to load older episodes.".to_string();
            return Task::none();
        };
        self.status = "Loading older episodes...".to_string();
        Task::perform(
            podcast::older_episodes(feed_url.clone(), key, secret),
            move |result| {
                Message::Podcast(PodcastMessage::OlderLoaded { feed_url, result })
            },
        )
    }

    fn store_older(
        &mut self,
        feed_url: String,
        result: Result<Vec<Episode>, String>,
    ) -> Task<Message> {
        let episodes = match result {
            Ok(episodes) => episodes,
            Err(error) => {
                if error.contains("rejected") {
                    self.podcasts.key_status = KeyStatus::Rejected;
                }
                self.status = error;
                return Task::none();
            }
        };
        let Some(show) = self.podcasts.favorite_mut(&feed_url) else {
            self.status = "That show is no longer a favorite.".to_string();
            return Task::none();
        };
        let before = show.episodes.len();
        show.episodes = podcast::merge_episodes(&show.episodes, &episodes);
        let added = show.episodes.len().saturating_sub(before);
        self.podcasts.older_loaded.insert(feed_url);
        self.podcasts.key_status = KeyStatus::Valid;
        self.remember_favorites();
        self.status = if added == 0 {
            "Podcast Index has no older episodes for this show.".to_string()
        } else {
            format!("Added {added} older episodes")
        };
        Task::none()
    }

    fn search_shows(&mut self) -> Task<Message> {
        let Some((key, secret)) =
            podcast::credentials(&self.podcasts.saved_key, &self.podcasts.saved_secret)
        else {
            self.tab = super::Tab::Settings;
            self.status = "Add a Podcast Index API key to search.".to_string();
            return Task::none();
        };
        let query = self.podcasts.query.clone();
        self.status = format!("Searching for \"{query}\"...").to_string();
        Task::perform(podcast::search(query, key, secret), |result| {
            Message::Podcast(PodcastMessage::SearchFinished(result))
        })
    }

    fn store_feed(&mut self, result: Result<Show, String>) -> Task<Message> {
        let fresh = match result {
            Ok(show) => show,
            Err(error) => {
                if !self.podcasts.favorites.is_empty() {
                    self.podcasts.offline = true;
                    self.status = "Offline - showing saved episodes".to_string();
                } else {
                    self.status = error;
                }
                return Task::none();
            }
        };
        let previous = self
            .podcasts
            .favorites
            .iter()
            .find(|show| show.feed_url == fresh.feed_url)
            .cloned();
        let mut fresh = fresh;
        let extra = previous
            .as_ref()
            .map(|show| show.episodes.clone())
            .unwrap_or_default();
        fresh.episodes = podcast::merge_episodes(&fresh.episodes, &extra);
        let feed_url = fresh.feed_url.clone();
        if let Some(slot) = self.podcasts.favorite_mut(&feed_url) {
            *slot = fresh;
        } else {
            self.podcasts.favorites.insert(0, fresh);
        }
        self.podcasts.selected = Some(feed_url);
        self.podcasts.offline = false;
        self.podcasts.rss_input.clear();
        self.remember_favorites();
        self.status = "Episodes updated.".to_string();
        Task::none()
    }

    fn play_episode(&mut self, feed_url: &str, guid: &str, play_next: bool) -> Task<Message> {
        let Some(track) = episode_track(&self.podcasts, feed_url, guid) else {
            self.status = "That episode is no longer in the list.".to_string();
            return Task::none();
        };
        let streamed = !track.is_persistable();
        if play_next {
            let was_empty = self.queue.is_empty();
            self.queue.insert_next(track);
            if was_empty {
                return self.start_current();
            }
            self.status = if streamed {
                "Will play next. Streamed episodes are skipped when the queue is saved.".to_string()
            } else {
                "Will play next.".to_string()
            };
            self.persist();
            return Task::none();
        }
        let index = self.queue.len();
        self.queue.push_many(vec![track]);
        let _ = self.queue.select_display(index);
        if streamed {
            self.status = "Streaming. Download the episode to keep it in a playlist.".to_string();
        }
        self.start_current()
    }

    fn start_download(&mut self, feed_url: &str, guid: &str) -> Task<Message> {
        let Some(show) = self
            .podcasts
            .favorites
            .iter()
            .find(|show| show.feed_url == feed_url)
        else {
            return Task::none();
        };
        let Some(episode) = show.episodes.iter().find(|episode| episode.guid == guid) else {
            return Task::none();
        };
        if episode.is_downloaded() {
            self.status = "That episode is already downloaded.".to_string();
            return Task::none();
        }
        let key = crate::podcast::PodcastState::progress_key(feed_url, guid);
        if self.podcasts.progress.contains_key(&key) {
            return Task::none();
        }
        let destination = podcast::episode_path(&show.title, episode);
        let episode = episode.clone();
        self.podcasts.progress.insert(key, 0.0);
        self.status = format!("Downloading \"{}\"...", episode.title);
        podcast::download_task(feed_url.to_string(), episode, destination)
    }

    fn finish_download(
        &mut self,
        feed_url: String,
        guid: String,
        result: Result<PathBuf, String>,
    ) -> Task<Message> {
        let key = crate::podcast::PodcastState::progress_key(&feed_url, &guid);
        self.podcasts.progress.remove(&key);
        match result {
            Ok(path) => {
                if let Some(show) = self.podcasts.favorite_mut(&feed_url) {
                    if let Some(episode) = show
                        .episodes
                        .iter_mut()
                        .find(|episode| episode.guid == guid)
                    {
                        episode.local_path = Some(path);
                    }
                }
                self.remember_favorites();
                self.status = "Download finished.".to_string();
            }
            Err(error) => self.status = error,
        }
        Task::none()
    }

    fn remove_download(&mut self, feed_url: &str, guid: &str) {
        let Some(show) = self.podcasts.favorite_mut(feed_url) else {
            return;
        };
        let Some(episode) = show
            .episodes
            .iter_mut()
            .find(|episode| episode.guid == guid)
        else {
            return;
        };
        if let Some(path) = episode.local_path.clone() {
            if let Err(error) = podcast::remove_download(&path) {
                self.status = format!("Could not remove the download: {error}");
                return;
            }
        }
        episode.local_path = None;
        self.remember_favorites();
        self.status = "Removed the download.".to_string();
    }

    fn remember_favorites(&mut self) {
        if let Err(error) = podcast::save_favorites(&self.podcasts.favorites) {
            self.status = format!("Could not save favorites: {error}");
        }
    }
}

fn fetch_feed(feed_url: String) -> Task<Message> {
    Task::perform(podcast::fetch_show(feed_url), |result| {
        Message::Podcast(PodcastMessage::FeedFinished { result })
    })
}

fn episode_track(
    podcasts: &crate::podcast::PodcastState,
    feed_url: &str,
    guid: &str,
) -> Option<Track> {
    let show = podcasts
        .favorites
        .iter()
        .find(|show| show.feed_url == feed_url)?;
    let episode = show.episodes.iter().find(|episode| episode.guid == guid)?;
    Some(track_for(show, episode))
}

fn track_for(show: &Show, episode: &Episode) -> Track {
    if let Some(path) = &episode.local_path {
        if path.is_file() {
            let mut track = Track::load(path);
            track.title = episode.title.clone();
            if track.artist.is_empty() {
                track.artist = show.title.clone();
            }
            track.stream_url = Some(episode.audio_url.clone());
            if track.duration == Duration::ZERO {
                track.duration = episode.duration();
            }
            return track;
        }
    }
    Track::from_stream(
        &episode.title,
        &show.title,
        &episode.audio_url,
        episode.duration(),
    )
}
