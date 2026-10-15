// ------------------------------------------------------------
// Module: playback
// Purpose: Play, skip, seek, load, and point a missing file at
//          its new location.
// ------------------------------------------------------------

use std::path::PathBuf;
use std::time::Duration;

use iced::Task;
use iced::widget::image;

use crate::config;
use crate::library::{self, Track};
use crate::queue::Step;

use super::{LoadMode, Message, State};

pub(crate) const MISSING_HINT: &str = "File not found - right-click > Locate file...";

impl State {
    pub(crate) fn on_tick(&mut self) -> Task<Message> {
        let snapshot = self.audio.snapshot();
        if self.seek_drag.is_none() {
            self.position = self.reported_position(snapshot.position);
        }
        self.sync_listening_memory();
        let playing = snapshot.active && !snapshot.paused && !snapshot.loading;
        if playing && self.listening_saved_at.elapsed() >= config::RESUME_SAVE_EVERY {
            self.flush_listening();
        }
        if snapshot.generation != self.seen_generation {
            self.seen_generation = snapshot.generation;
            self.end_armed = snapshot.active;
            if let Some(error) = snapshot.error {
                self.note_open_error(error);
            } else if !snapshot.loading {
                self.status.clear();
            }
            return Task::none();
        }
        if self.end_armed && snapshot.active && snapshot.empty && !snapshot.paused {
            self.end_armed = false;
            return self.skip_next(true);
        }
        Task::none()
    }

    pub(crate) fn toggle_play(&mut self) -> Task<Message> {
        let snapshot = self.audio.snapshot();
        if snapshot.loading {
            return Task::none();
        }
        if !snapshot.active || snapshot.empty {
            return self.start_current();
        }
        if snapshot.paused {
            self.audio.resume();
        } else {
            self.sync_listening_memory();
            self.flush_listening();
            self.audio.pause();
        }
        Task::none()
    }

    pub(crate) fn skip_next(&mut self, automatic: bool) -> Task<Message> {
        match self.queue.go_next(automatic) {
            Step::Play => self.start_current(),
            Step::Restart => self.start_from_beginning(),
            Step::End => {
                self.stop_playback();
                self.persist();
                Task::none()
            }
        }
    }

    pub(crate) fn skip_previous(&mut self) -> Task<Message> {
        match self.queue.go_previous(self.position) {
            Step::Play => self.start_current(),
            Step::Restart => {
                let snapshot = self.audio.snapshot();
                if snapshot.active && !snapshot.empty {
                    self.audio.seek(Duration::ZERO);
                    self.position = Duration::ZERO;
                    self.resume_hold = Some(Duration::ZERO);
                    self.resume_hold_since = std::time::Instant::now();
                    if let Some(url) = self.resume_url.clone() {
                        self.podcasts.listening.positions.remove(&url);
                    }
                    self.flush_listening();
                    Task::none()
                } else {
                    self.start_from_beginning()
                }
            }
            Step::End => Task::none(),
        }
    }

    pub(crate) fn start_current(&mut self) -> Task<Message> {
        self.begin_playback(true)
    }

    // --------------------------------------------------------
    // Method: start_from_beginning
    // Purpose: Restarts the current track. Repeat-one and the
    //          previous button use this so a saved position is
    //          not applied on top of an intentional restart.
    // --------------------------------------------------------
    pub(crate) fn start_from_beginning(&mut self) -> Task<Message> {
        self.begin_playback(false)
    }

    fn begin_playback(&mut self, resume: bool) -> Task<Message> {
        let next_url = self
            .queue
            .current()
            .and_then(|track| track.stream_url.clone());
        let same_episode = self.resume_url == next_url && next_url.is_some();
        if !same_episode {
            self.sync_listening_memory();
            self.flush_listening();
        }
        if !resume && let Some(url) = &next_url {
            self.podcasts.listening.positions.remove(url);
        }
        let Some(track) = self.queue.current() else {
            self.stop_playback();
            self.cover = None;
            return Task::none();
        };
        let cover = track.cover.clone();
        let path = track.path.clone();
        let stream_url = track.stream_url.clone();
        let duration = track.duration;
        let local = track.plays_from_file();
        let start_at = stream_url
            .as_ref()
            .and_then(|url| self.podcasts.listening.positions.get(url).copied())
            .and_then(|secs| crate::podcast::resume_point(secs, duration))
            .unwrap_or(Duration::ZERO);
        self.cover = cover.map(image::Handle::from_bytes);
        self.position = start_at;
        self.seek_drag = None;
        self.resume_url = stream_url.clone();
        self.resume_duration = duration;
        self.resume_hold = (start_at > Duration::ZERO).then_some(start_at);
        self.resume_hold_since = std::time::Instant::now();
        self.podcasts.resume_skip_sync = false;
        if start_at > Duration::ZERO {
            self.status = format!("Resuming at {}", crate::util::format_time(start_at));
        }
        if local {
            if let Some(track) = self.queue.current_mut() {
                track.missing = false;
            }
            self.audio.play(path, start_at);
            self.persist();
            return Task::none();
        }
        if let Some(url) = stream_url {
            if start_at == Duration::ZERO {
                self.status = "Opening stream...".to_string();
            }
            self.audio.play_stream(url, start_at);
            self.persist();
            return Task::none();
        }
        if let Some(track) = self.queue.current_mut() {
            track.missing = true;
        }
        self.status = MISSING_HINT.to_string();
        self.stop_playback();
        self.persist();
        Task::none()
    }

    pub(crate) fn stop_playback(&mut self) {
        self.sync_listening_memory();
        self.flush_listening();
        self.resume_url = None;
        self.resume_hold = None;
        self.audio.stop();
        self.end_armed = false;
        self.position = Duration::ZERO;
    }

    pub(crate) fn release_seek(&mut self) -> Task<Message> {
        if let Some(value) = self.seek_drag.take() {
            let position = Duration::from_secs_f32(value.max(0.0));
            self.position = position;
            self.resume_hold = None;
            self.audio.seek(position);
            self.sync_listening_memory();
            self.flush_listening();
        }
        Task::none()
    }

    // --------------------------------------------------------
    // Method: reported_position
    // Purpose: Keeps the resumed playhead on screen until the
    //          audio thread reports a matching position.
    // --------------------------------------------------------
    fn reported_position(&mut self, reported: Duration) -> Duration {
        let Some(hold) = self.resume_hold else {
            return reported;
        };
        let caught_up = if hold == Duration::ZERO {
            reported < Duration::from_secs(2)
        } else {
            reported > Duration::from_secs(1) && reported + Duration::from_secs(2) >= hold
        };
        if caught_up || self.resume_hold_since.elapsed() >= Duration::from_secs(3) {
            self.resume_hold = None;
            reported
        } else {
            hold
        }
    }

    // --------------------------------------------------------
    // Method: sync_listening_memory
    // Purpose: Copies the playhead into the episode map. Disk
    //          writes happen on a timer and when playback changes.
    // --------------------------------------------------------
    pub(crate) fn sync_listening_memory(&mut self) {
        if self.resume_hold.is_some() || self.podcasts.resume_skip_sync {
            return;
        }
        let Some(url) = self.resume_url.clone() else {
            return;
        };
        crate::podcast::note_listen(
            &mut self.podcasts.listening,
            &url,
            self.position,
            self.resume_duration,
        );
    }

    pub(crate) fn flush_listening(&mut self) {
        self.listening_saved_at = std::time::Instant::now();
        let _ = crate::podcast::save_listening(&self.podcasts.listening);
    }

    pub(crate) fn seek_by(&mut self, delta: f32) -> Task<Message> {
        let total = self
            .queue
            .current()
            .map(|track| track.duration.as_secs_f32())
            .unwrap_or(0.0);
        let next = (self.position.as_secs_f32() + delta).clamp(0.0, total.max(0.0));
        let position = Duration::from_secs_f32(next);
        self.position = position;
        self.seek_drag = None;
        self.audio.seek(position);
        Task::none()
    }

    pub(crate) fn remove_index(&mut self, index: usize) -> Task<Message> {
        self.menu = None;
        let removed_current = self.queue.remove_display(index);
        if self.queue.is_empty() {
            self.stop_playback();
            self.cover = None;
        } else if removed_current {
            return self.start_current();
        }
        self.persist();
        Task::none()
    }

    pub(crate) fn finish_load(&mut self, tracks: Vec<Track>, mode: LoadMode) -> Task<Message> {
        if tracks.is_empty() {
            self.status = "No supported audio files found.".to_string();
            return Task::none();
        }
        match mode {
            LoadMode::ReplaceAndPlay => {
                self.queue.clear();
                self.queue.push_many(tracks);
                let _ = self.queue.select_display(0);
                self.start_current()
            }
            LoadMode::Append { interrupt } => {
                let start = self.queue.len();
                let idle = !self.audio.snapshot().active;
                self.queue.push_many(tracks);
                if interrupt || idle {
                    let _ = self.queue.select_display(start);
                    self.start_current()
                } else {
                    self.status = "Added tracks to the queue.".to_string();
                    self.persist();
                    Task::none()
                }
            }
            LoadMode::Restore { index } => {
                self.queue.clear();
                self.queue.push_many(tracks);
                if let Some(index) = index {
                    let last = self.queue.len().saturating_sub(1);
                    let _ = self.queue.select_display(index.min(last));
                }
                self.refresh_cover();
                self.persist();
                Task::none()
            }
        }
    }

    pub(crate) fn refresh_cover(&mut self) {
        self.cover = self.queue.current().and_then(|track| {
            track
                .cover
                .as_ref()
                .map(|bytes| image::Handle::from_bytes(bytes.clone()))
        });
    }

    // --------------------------------------------------------
    // Method: relocate
    // Purpose: Swaps a moved file for the path the user picked,
    //          including every saved playlist that used the old path.
    // --------------------------------------------------------
    pub(crate) fn relocate(&mut self, index: usize, path: PathBuf) -> Task<Message> {
        let Some(old) = self
            .queue
            .tracks()
            .get(index)
            .map(|track| track.path.clone())
        else {
            return Task::none();
        };
        if old.as_os_str().is_empty() {
            self.status = "Download the episode instead of locating a file.".to_string();
            return Task::none();
        }
        let loaded = Track::load(&path);
        let current_path = self.queue.current().map(|track| track.path.clone());
        let rows = self.queue.retarget(&old, &loaded);
        let playlists = library::replace_path(&config::playlist_dir(), &old, &path).unwrap_or(0);
        self.status = format!("Updated {rows} queued track(s) and {playlists} playlist(s).");
        self.persist();
        if current_path.as_deref() == Some(old.as_path()) {
            self.start_current()
        } else {
            Task::none()
        }
    }

    fn note_open_error(&mut self, error: String) {
        let stream = self
            .queue
            .current()
            .is_some_and(|track| track.stream_url.is_some() && !track.plays_from_file());
        let missing_file = self
            .queue
            .current()
            .is_some_and(|track| track.stream_url.is_none() && !track.plays_from_file());
        if !stream && (missing_file || error.starts_with("Could not open")) {
            if let Some(track) = self.queue.current_mut() {
                if !track.plays_from_file() {
                    track.missing = true;
                    self.status = MISSING_HINT.to_string();
                    return;
                }
            }
        }
        self.status = error;
    }
}
