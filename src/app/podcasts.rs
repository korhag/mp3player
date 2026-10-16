// ------------------------------------------------------------
// Module: podcasts
// Purpose: Search, favorites, streaming playback, and downloads
//          for the Podcasts tab.
// ------------------------------------------------------------

use std::path::PathBuf;
use std::time::Duration;

use iced::Task;
use iced::widget::pane_grid;

use crate::library::Track;
use crate::podcast::{
    self, DragGesture, DragItem, DropSlot, DropTarget, Episode, FavoriteItem, GroupChange,
    KeyStatus, Show,
};

use super::Message;
use super::State;

#[derive(Debug, Clone)]
pub enum PodcastMessage {
    QueryChanged(String),
    Search,
    SearchFinished(Result<Vec<Show>, String>),
    FeedFinished {
        result: Result<Show, String>,
        preview: bool,
    },
    OpenShow(String),
    PreviewShow(String),
    RefreshShow(String),
    ToggleFavorite(String),
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
        from_start: bool,
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
    PaneDragged(pane_grid::DragEvent),
    PaneResized(pane_grid::ResizeEvent),
    ResetLayout,
    EpisodeFilterChanged(String),
    SetEpisodesOnly(bool),
    SelectEpisode {
        feed_url: String,
        guid: String,
    },
    DragPressed {
        item: DragItem,
        x: f32,
        y: f32,
    },
    DragTarget(DropTarget),
    DragLeave(DropTarget),
    DragReleased,
    ToggleGroup(usize),
    StartGroupRename(usize),
    GroupRenameInput(String),
    ConfirmGroupRename,
    CancelGroupRename,
    Ungroup(usize),
    MoveGroup {
        index: usize,
        delta: isize,
    },
    RemoveFromGroup(String),
    MarkPlayed {
        feed_url: String,
        guid: String,
        played: bool,
    },
    ArtworkReady {
        url: String,
        image: Option<(u32, u32, Vec<u8>)>,
    },
}

impl State {
    pub(crate) fn podcast(&mut self, message: PodcastMessage) -> Task<Message> {
        match message {
            PodcastMessage::QueryChanged(query) => {
                self.podcasts.query = query;
                Task::none()
            }
            PodcastMessage::Search => self.search_or_preview(),
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
            PodcastMessage::FeedFinished { result, preview } => {
                if preview {
                    self.store_preview(result)
                } else {
                    self.store_feed(result)
                }
            }
            PodcastMessage::OpenShow(feed_url) => self.open_show(feed_url),
            PodcastMessage::PreviewShow(feed_url) => self.preview_show(feed_url),
            PodcastMessage::RefreshShow(feed_url) => {
                self.menu = None;
                self.status = "Reading the feed...".to_string();
                fetch_feed(feed_url, false)
            }
            PodcastMessage::ToggleFavorite(feed_url) => self.toggle_favorite(feed_url),
            PodcastMessage::RemoveShow(feed_url) => self.remove_show(feed_url),
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
            PodcastMessage::PlayEpisode {
                feed_url,
                guid,
                from_start,
            } => {
                self.menu = None;
                self.play_episode(&feed_url, &guid, false, from_start)
            }
            PodcastMessage::QueueEpisodeNext { feed_url, guid } => {
                self.menu = None;
                self.play_episode(&feed_url, &guid, true, false)
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
            PodcastMessage::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.podcasts.panes.drop(pane, target);
                let _ = podcast::save_panes(&self.podcasts.panes);
                Task::none()
            }
            PodcastMessage::PaneDragged(_) => Task::none(),
            PodcastMessage::PaneResized(event) => {
                self.podcasts.panes.resize(event.split, event.ratio);
                let _ = podcast::save_panes(&self.podcasts.panes);
                Task::none()
            }
            PodcastMessage::ResetLayout => {
                self.podcasts.panes = podcast::default_panes();
                let _ = podcast::save_panes(&self.podcasts.panes);
                self.status = "Restored the podcast layout.".to_string();
                Task::none()
            }
            PodcastMessage::EpisodeFilterChanged(filter) => {
                self.podcasts.episode_filter = filter;
                Task::none()
            }
            PodcastMessage::SetEpisodesOnly(episodes_only) => {
                self.podcasts.episodes_only = episodes_only;
                Task::none()
            }
            PodcastMessage::SelectEpisode { feed_url, guid } => {
                self.podcasts.selected_episode = Some((feed_url, guid));
                Task::none()
            }
            PodcastMessage::DragPressed { item, x, y } => {
                self.podcasts.drag = Some(DragGesture::press(item, x, y));
                self.podcasts.drop_target = None;
                Task::none()
            }
            PodcastMessage::DragTarget(target) => {
                if self.podcasts.drag.is_some() {
                    self.podcasts.drop_target = Some(target);
                }
                Task::none()
            }
            PodcastMessage::DragLeave(target) => {
                if self.podcasts.drop_target.as_ref() == Some(&target) {
                    self.podcasts.drop_target = None;
                }
                Task::none()
            }
            PodcastMessage::DragReleased => self.release_drag(),
            PodcastMessage::ToggleGroup(index) => {
                podcast::toggle_collapsed(&mut self.podcasts.groups, index);
                self.remember_groups();
                Task::none()
            }
            PodcastMessage::StartGroupRename(index) => {
                self.menu = None;
                if let Some(FavoriteItem::Group { name, .. }) = self.podcasts.groups.get(index) {
                    self.podcasts.group_rename_text = name.clone();
                    self.podcasts.rename_group = Some(index);
                }
                Task::none()
            }
            PodcastMessage::GroupRenameInput(text) => {
                self.podcasts.group_rename_text = text;
                Task::none()
            }
            PodcastMessage::ConfirmGroupRename => {
                if let Some(index) = self.podcasts.rename_group.take() {
                    let name = self.podcasts.group_rename_text.clone();
                    if podcast::rename_group(&mut self.podcasts.groups, index, &name) {
                        self.remember_groups();
                    }
                }
                Task::none()
            }
            PodcastMessage::CancelGroupRename => {
                self.podcasts.rename_group = None;
                Task::none()
            }
            PodcastMessage::Ungroup(index) => {
                self.menu = None;
                if podcast::ungroup(&mut self.podcasts.groups, index) {
                    self.remember_groups();
                }
                Task::none()
            }
            PodcastMessage::MoveGroup { index, delta } => {
                self.menu = None;
                if podcast::move_group_by(&mut self.podcasts.groups, index, delta) {
                    self.remember_groups();
                }
                Task::none()
            }
            PodcastMessage::RemoveFromGroup(feed_url) => {
                self.menu = None;
                if podcast::remove_from_group(&mut self.podcasts.groups, &feed_url) {
                    self.remember_groups();
                }
                Task::none()
            }
            PodcastMessage::MarkPlayed {
                feed_url,
                guid,
                played,
            } => self.mark_played(&feed_url, &guid, played),
            PodcastMessage::ArtworkReady { url, image } => {
                self.podcasts.artwork_pending.remove(&url);
                if let Some((width, height, pixels)) = image {
                    self.artwork.insert(
                        url,
                        iced::widget::image::Handle::from_rgba(width, height, pixels),
                    );
                }
                Task::none()
            }
        }
    }

    // --------------------------------------------------------
    // Method: request_artwork
    // Purpose: Fetches a show image once. Empty URLs and images
    //          already on screen are ignored.
    // --------------------------------------------------------
    pub(crate) fn request_artwork(&mut self, url: String) -> Task<Message> {
        let url = url.trim().to_string();
        if url.is_empty()
            || self.artwork.contains_key(&url)
            || self.podcasts.artwork_pending.contains(&url)
        {
            return Task::none();
        }
        self.podcasts.artwork_pending.insert(url.clone());
        Task::perform(podcast::fetch_artwork(url.clone()), |image| {
            Message::Podcast(PodcastMessage::ArtworkReady {
                url,
                image: image.ok(),
            })
        })
    }

    fn search_or_preview(&mut self) -> Task<Message> {
        let query = self.podcasts.query.trim().to_string();
        if let Some(url) = podcast::feed_url(&query) {
            self.status = "Reading the feed...".to_string();
            return fetch_feed(url, true);
        }
        self.search_shows()
    }

    fn open_show(&mut self, feed_url: String) -> Task<Message> {
        self.menu = None;
        self.podcasts.selected = Some(feed_url.clone());
        self.podcasts.episode_filter.clear();
        self.podcasts.selected_episode = None;
        self.artwork_for(&feed_url)
    }

    fn preview_show(&mut self, feed_url: String) -> Task<Message> {
        self.menu = None;
        if self.podcasts.is_favorite(&feed_url) {
            return self.open_show(feed_url);
        }
        self.status = "Reading the feed...".to_string();
        fetch_feed(feed_url, true)
    }

    fn artwork_for(&mut self, feed_url: &str) -> Task<Message> {
        let url = self
            .podcasts
            .find_show(feed_url)
            .and_then(|show| show.image_url.clone())
            .unwrap_or_default();
        self.request_artwork(url)
    }

    fn toggle_favorite(&mut self, feed_url: String) -> Task<Message> {
        if self.podcasts.is_favorite(&feed_url) {
            return self.remove_show(feed_url);
        }
        let Some(show) = self
            .podcasts
            .preview
            .clone()
            .filter(|show| show.feed_url == feed_url)
        else {
            self.status = "Reading the feed...".to_string();
            return fetch_feed(feed_url, false);
        };
        self.podcasts.favorites.insert(0, show);
        self.podcasts
            .groups
            .insert(0, FavoriteItem::Show(feed_url.clone()));
        self.podcasts.groups = podcast::reconcile(
            std::mem::take(&mut self.podcasts.groups),
            &self.podcasts.favorites,
        );
        if self
            .podcasts
            .preview
            .as_ref()
            .map(|show| show.feed_url.as_str())
            == Some(feed_url.as_str())
        {
            self.podcasts.preview = None;
        }
        self.podcasts.selected = Some(feed_url);
        self.remember_favorites();
        self.remember_groups();
        self.status = "Added to favorites.".to_string();
        Task::none()
    }

    fn remove_show(&mut self, feed_url: String) -> Task<Message> {
        self.menu = None;
        let already_preview = self
            .podcasts
            .preview
            .as_ref()
            .is_some_and(|show| show.feed_url == feed_url);
        if self.podcasts.selected.as_deref() == Some(feed_url.as_str())
            && !already_preview
            && let Some(show) = self
                .podcasts
                .favorites
                .iter()
                .find(|show| show.feed_url == feed_url)
                .cloned()
        {
            self.podcasts.preview = Some(show);
        }
        self.podcasts
            .favorites
            .retain(|show| show.feed_url != feed_url);
        podcast::forget_show(&mut self.podcasts.groups, &feed_url);
        if self.podcasts.selected.as_deref() == Some(feed_url.as_str())
            && self.podcasts.find_show(&feed_url).is_none()
        {
            self.podcasts.selected = self
                .podcasts
                .favorites
                .first()
                .map(|show| show.feed_url.clone());
        }
        self.remember_favorites();
        self.remember_groups();
        self.status = "Removed the show from favorites.".to_string();
        Task::none()
    }

    fn release_drag(&mut self) -> Task<Message> {
        let Some(gesture) = self.podcasts.drag.take() else {
            return Task::none();
        };
        let target = self.podcasts.drop_target.take();
        if !gesture.armed {
            if let DragItem::Show(feed_url) = gesture.item {
                return self.open_show(feed_url);
            }
            return Task::none();
        }
        let Some(target) = target else {
            return Task::none();
        };
        let changed = match (gesture.item, target) {
            (DragItem::Show(feed), DropTarget::OnShow(other)) => {
                match podcast::group_with(&mut self.podcasts.groups, &feed, &other) {
                    Some(GroupChange::Created(index)) => {
                        self.podcasts.group_rename_text = "New group".to_string();
                        self.podcasts.rename_group = Some(index);
                        true
                    }
                    Some(GroupChange::Joined(_)) => true,
                    None => false,
                }
            }
            (DragItem::Show(feed), DropTarget::OnGroup(index)) => {
                let Some(before) = self.podcasts.groups.get(index).and_then(|item| match item {
                    FavoriteItem::Group { shows, .. } => Some(shows.len()),
                    FavoriteItem::Show(_) => None,
                }) else {
                    return Task::none();
                };
                podcast::move_show(
                    &mut self.podcasts.groups,
                    &feed,
                    DropSlot::InGroup {
                        group: index,
                        before,
                    },
                )
            }
            (DragItem::Show(feed), DropTarget::Slot(slot)) => {
                podcast::move_show(&mut self.podcasts.groups, &feed, slot)
            }
            (DragItem::Group(index), DropTarget::Slot(slot)) => {
                podcast::move_group(&mut self.podcasts.groups, index, slot)
            }
            (DragItem::Group(index), DropTarget::OnGroup(other)) if index != other => {
                podcast::move_group(&mut self.podcasts.groups, index, DropSlot::Before(other))
            }
            (DragItem::Group(index), DropTarget::OnShow(feed)) => {
                let Some(at) = podcast::container_index(&self.podcasts.groups, &feed) else {
                    return Task::none();
                };
                podcast::move_group(&mut self.podcasts.groups, index, DropSlot::Before(at))
            }
            _ => false,
        };
        if changed {
            self.remember_groups();
        }
        Task::none()
    }

    fn mark_played(&mut self, feed_url: &str, guid: &str, played: bool) -> Task<Message> {
        self.menu = None;
        let Some(url) = self
            .podcasts
            .find_show(feed_url)
            .and_then(|show| show.episodes.iter().find(|episode| episode.guid == guid))
            .map(|episode| episode.audio_url.clone())
        else {
            return Task::none();
        };
        podcast::mark_played(&mut self.podcasts.listening, &url, played);
        if self.resume_url.as_deref() == Some(url.as_str()) {
            self.podcasts.resume_skip_sync = true;
        }
        self.flush_listening();
        self.status = if played {
            "Marked as finished.".to_string()
        } else {
            "Marked as not finished.".to_string()
        };
        Task::none()
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
        if !self.podcasts.is_favorite(&feed_url) {
            self.status = "Favorite the show to load older episodes.".to_string();
            return Task::none();
        }
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
            move |result| Message::Podcast(PodcastMessage::OlderLoaded { feed_url, result }),
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

    fn store_preview(&mut self, result: Result<Show, String>) -> Task<Message> {
        let fresh = match result {
            Ok(show) => show,
            Err(error) => {
                self.status = error;
                return Task::none();
            }
        };
        if self.podcasts.is_favorite(&fresh.feed_url) {
            return self.store_feed(Ok(fresh));
        }
        let artwork = fresh.image_url.clone().unwrap_or_default();
        let feed_url = fresh.feed_url.clone();
        self.podcasts.selected = Some(feed_url);
        self.podcasts.episode_filter.clear();
        self.podcasts.selected_episode = None;
        self.podcasts.preview = Some(fresh);
        self.podcasts.offline = false;
        self.status = "Showing episodes. Favorite the show to keep them.".to_string();
        self.request_artwork(artwork)
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
        if fresh.description.is_empty()
            && let Some(previous) = &previous
        {
            fresh.description = previous.description.clone();
        }
        let artwork = fresh.image_url.clone().unwrap_or_default();
        let feed_url = fresh.feed_url.clone();
        let inserted = if let Some(slot) = self.podcasts.favorite_mut(&feed_url) {
            *slot = fresh;
            false
        } else {
            self.podcasts.favorites.insert(0, fresh);
            true
        };
        if inserted {
            self.podcasts
                .groups
                .insert(0, FavoriteItem::Show(feed_url.clone()));
            self.podcasts.groups = podcast::reconcile(
                std::mem::take(&mut self.podcasts.groups),
                &self.podcasts.favorites,
            );
            self.remember_groups();
        }
        if self
            .podcasts
            .preview
            .as_ref()
            .is_some_and(|show| show.feed_url == feed_url)
        {
            self.podcasts.preview = None;
        }
        self.podcasts.selected = Some(feed_url);
        self.podcasts.episode_filter.clear();
        self.podcasts.offline = false;
        self.remember_favorites();
        self.status = "Episodes updated.".to_string();
        self.request_artwork(artwork)
    }

    fn play_episode(
        &mut self,
        feed_url: &str,
        guid: &str,
        play_next: bool,
        from_start: bool,
    ) -> Task<Message> {
        let Some(track) = episode_track(&self.podcasts, feed_url, guid) else {
            self.status = "That episode is no longer in the list.".to_string();
            return Task::none();
        };
        if from_start {
            if let Some(url) = &track.stream_url {
                self.podcasts.listening.positions.remove(url);
                self.podcasts.listening.played.remove(url);
            }
            self.flush_listening();
        }
        let streamed = !track.is_persistable();
        if play_next {
            if let Some(index) = queue_index(&self.queue, &track) {
                self.play_next_row(index);
                return Task::none();
            }
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
        if self.same_as_current(&track) {
            return self.replay_current(from_start);
        }
        if let Some(index) = queue_index(&self.queue, &track) {
            let _ = self.queue.select_display(index);
            return if from_start {
                self.start_from_beginning()
            } else {
                self.start_current()
            };
        }
        let index = self.queue.len();
        self.queue.push_many(vec![track]);
        let _ = self.queue.select_display(index);
        if streamed {
            self.status = "Streaming. Download the episode to keep it in a playlist.".to_string();
        }
        if from_start {
            self.start_from_beginning()
        } else {
            self.start_current()
        }
    }

    fn replay_current(&mut self, from_start: bool) -> Task<Message> {
        let snapshot = self.audio.snapshot();
        if from_start {
            self.position = Duration::ZERO;
            self.resume_hold = Some(Duration::ZERO);
            self.resume_hold_since = std::time::Instant::now();
            self.audio.seek(Duration::ZERO);
            if snapshot.paused && snapshot.active {
                self.audio.resume();
            } else if !snapshot.active || snapshot.empty {
                return self.start_from_beginning();
            }
            self.status = "Playing from the start.".to_string();
            return Task::none();
        }
        if snapshot.active && !snapshot.empty {
            if snapshot.paused || snapshot.loading {
                if snapshot.paused {
                    self.audio.resume();
                }
                return Task::none();
            }
            return Task::none();
        }
        self.start_current()
    }

    fn start_download(&mut self, feed_url: &str, guid: &str) -> Task<Message> {
        if !self.podcasts.is_favorite(feed_url) {
            self.status = "Favorite the show before downloading.".to_string();
            return Task::none();
        }
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
                if let Some(show) = self.podcasts.favorite_mut(&feed_url)
                    && let Some(episode) = show
                        .episodes
                        .iter_mut()
                        .find(|episode| episode.guid == guid)
                {
                    episode.local_path = Some(path);
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
        if let Some(path) = episode.local_path.clone()
            && let Err(error) = podcast::remove_download(&path)
        {
            self.status = format!("Could not remove the download: {error}");
            return;
        }
        episode.local_path = None;
        self.remember_favorites();
        self.status = "Removed the download.".to_string();
    }

    // --------------------------------------------------------
    // Method: learn_playback_duration
    // Purpose: Some feeds omit itunes:duration. Once the decoder
    //          reports a length, the seek bar and the episode list
    //          can use it. A length we already have is not replaced.
    // --------------------------------------------------------
    pub(crate) fn learn_playback_duration(&mut self, learned: Option<Duration>) {
        let Some(track) = self.queue.current() else {
            return;
        };
        let Some(secs) = podcast::apply_learned_duration(track.duration.as_secs(), learned) else {
            return;
        };
        let url = track.stream_url.clone();
        if let Some(track) = self.queue.current_mut() {
            track.duration = Duration::from_secs(secs);
        }
        self.resume_duration = Duration::from_secs(secs);
        let Some(url) = url else {
            return;
        };
        let saved = podcast::store_learned_duration(&mut self.podcasts.favorites, &url, learned);
        if let Some(preview) = self.podcasts.preview.as_mut() {
            let _ = podcast::store_learned_duration(std::slice::from_mut(preview), &url, learned);
        }
        if saved {
            self.remember_favorites();
        }
    }

    fn remember_favorites(&mut self) {
        if let Err(error) = podcast::save_favorites(&self.podcasts.favorites) {
            self.status = format!("Could not save favorites: {error}");
        }
    }

    fn remember_groups(&mut self) {
        if let Err(error) = podcast::save_groups(&self.podcasts.groups) {
            self.status = format!("Could not save groups: {error}");
        }
    }

    fn same_as_current(&self, track: &Track) -> bool {
        self.queue
            .current()
            .is_some_and(|current| same_track(current, track))
    }
}

fn fetch_feed(feed_url: String, preview: bool) -> Task<Message> {
    Task::perform(podcast::fetch_show(feed_url), move |result| {
        Message::Podcast(PodcastMessage::FeedFinished { result, preview })
    })
}

fn episode_track(
    podcasts: &crate::podcast::PodcastState,
    feed_url: &str,
    guid: &str,
) -> Option<Track> {
    let show = podcasts.find_show(feed_url)?;
    let episode = show.episodes.iter().find(|episode| episode.guid == guid)?;
    Some(track_for(show, episode))
}

fn queue_index(queue: &crate::queue::Queue, track: &Track) -> Option<usize> {
    queue
        .tracks()
        .iter()
        .position(|queued| same_track(queued, track))
}

fn same_track(queued: &Track, track: &Track) -> bool {
    match (&queued.stream_url, &track.stream_url) {
        (Some(left), Some(right)) if left == right => true,
        _ => !track.path.as_os_str().is_empty() && queued.path == track.path,
    }
}

fn track_for(show: &Show, episode: &Episode) -> Track {
    if let Some(path) = &episode.local_path
        && path.is_file()
    {
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
    Track::from_stream(
        &episode.title,
        &show.title,
        &episode.audio_url,
        episode.duration(),
    )
}
