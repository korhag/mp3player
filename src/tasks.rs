// ------------------------------------------------------------
// Module: tasks
// Purpose: File dialogs, playlist commands, keyboard shortcuts,
//          and the single-instance listener. Kept apart from the
//          window state so that file stays readable.
// ------------------------------------------------------------

use std::path::PathBuf;

use iced::event::{self, Event};
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::mouse;
use iced::window;
use iced::{Subscription, Task};

use crate::app::{LoadMode, Message, State};
use crate::config;
use crate::ipc;
use crate::library::{self, Track};

// ------------------------------------------------------------
// Function: load_tracks
// Purpose: Reads metadata off the UI thread, then returns tracks.
// ------------------------------------------------------------
pub fn load_tracks(paths: Vec<PathBuf>, mode: LoadMode) -> Task<Message> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                library::collect_audio_paths(&paths)
                    .into_iter()
                    .map(|path| Track::load(&path))
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default()
        },
        move |tracks| Message::TracksLoaded { tracks, mode },
    )
}

// ------------------------------------------------------------
// Function: pick_audio
// Purpose: Opens a file or folder dialog for supported audio.
// ------------------------------------------------------------
pub fn pick_audio(folder: bool) -> Task<Message> {
    let extensions = config::EXTENSIONS.to_vec();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let dialog = rfd::FileDialog::new();
                if folder {
                    dialog.pick_folder().map(|path| vec![path])
                } else {
                    dialog.add_filter("Audio", &extensions).pick_files()
                }
            })
            .await
            .ok()
            .flatten()
        },
        Message::PathsPicked,
    )
}

// ------------------------------------------------------------
// Function: shortcuts_and_drops
// Purpose: Keyboard shortcuts plus drag-and-drop and close.
//          Keys are ignored while a text field has them.
// ------------------------------------------------------------
pub fn shortcuts_and_drops(
    event: Event,
    status: event::Status,
    _id: iced::window::Id,
) -> Option<Message> {
    match event {
        Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Some(Message::CursorMoved(position))
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            Some(Message::Podcast(crate::app::PodcastMessage::DragReleased))
        }
        Event::Window(window::Event::FileDropped(path)) => Some(Message::FilesDropped(path)),
        Event::Window(window::Event::CloseRequested) => Some(Message::WindowClose),
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Named(Named::Escape),
            ..
        }) => Some(Message::Escape),
        Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
            if status == event::Status::Ignored =>
        {
            map_shortcut(key, modifiers)
        }
        _ => None,
    }
}

fn map_shortcut(key: Key, modifiers: Modifiers) -> Option<Message> {
    let plain = !modifiers.control() && !modifiers.alt() && !modifiers.logo();
    match key {
        Key::Named(Named::Space) if plain => Some(Message::TogglePlay),
        Key::Named(Named::ArrowRight) if modifiers.control() => Some(Message::Next),
        Key::Named(Named::ArrowLeft) if modifiers.control() => Some(Message::Previous),
        Key::Named(Named::ArrowRight) if plain => Some(Message::SkipForward),
        Key::Named(Named::ArrowLeft) if plain => Some(Message::SkipBack),
        Key::Named(Named::ArrowUp) if plain => Some(Message::VolumeBy(0.05)),
        Key::Named(Named::ArrowDown) if plain => Some(Message::VolumeBy(-0.05)),
        Key::Character(character) if plain && eq_letter(&character, "s") => {
            Some(Message::ToggleShuffle)
        }
        Key::Character(character) if plain && eq_letter(&character, "r") => {
            Some(Message::CycleRepeat)
        }
        _ => None,
    }
}

fn eq_letter(character: &impl AsRef<str>, letter: &str) -> bool {
    character.as_ref().eq_ignore_ascii_case(letter)
}

// ------------------------------------------------------------
// Function: ipc_subscription
// Purpose: Delivers files sent by later "Open with" processes.
// ------------------------------------------------------------
pub fn ipc_subscription() -> Subscription<Message> {
    Subscription::run(ipc_stream)
}

fn ipc_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(8, async |mut sender| {
        let Some(mut receiver) = ipc::take_receiver() else {
            std::future::pending::<()>().await;
            return;
        };
        while let Some(paths) = receiver.recv().await {
            if sender.try_send(Message::ExternalPaths(paths)).is_err() {
                break;
            }
        }
    })
}

impl State {
    // --------------------------------------------------------
    // Method: save_playlist
    // Purpose: Writes the current queue as an M3U8 playlist.
    // --------------------------------------------------------
    pub(crate) fn save_playlist(&mut self) {
        let name = self.playlist_name.trim().to_string();
        if name.is_empty() {
            self.status = "Name the playlist first.".to_string();
            return;
        }
        let tracks: Vec<Track> = self
            .queue
            .tracks()
            .iter()
            .filter(|track| track.is_persistable())
            .cloned()
            .collect();
        let skipped = self.queue.len() - tracks.len();
        if tracks.is_empty() {
            self.status = if skipped > 0 {
                "Streamed episodes are not saved. Download them first.".to_string()
            } else {
                "Add tracks before saving a playlist.".to_string()
            };
            return;
        }
        match library::save(&config::playlist_dir(), &name, &tracks) {
            Ok(_) => {
                self.status = if skipped > 0 {
                    format!(
                        "Saved playlist \"{name}\". Skipped {skipped} streamed episode(s). Download them to keep them."
                    )
                } else {
                    format!("Saved playlist \"{name}\".")
                };
                self.refresh_playlists();
            }
            Err(error) => self.status = format!("Could not save playlist: {error}"),
        }
    }

    // --------------------------------------------------------
    // Method: load_playlist
    // Purpose: Replaces the queue with a saved playlist and plays it.
    // --------------------------------------------------------
    pub(crate) fn load_playlist(&mut self, name: String) -> Task<Message> {
        match library::load(&config::playlist_dir(), &name) {
            Ok(paths) => {
                if paths.is_empty() {
                    self.status = format!("\"{name}\" has no tracks.");
                    Task::none()
                } else {
                    let missing = paths.iter().filter(|path| !path.is_file()).count();
                    self.status = if missing > 0 {
                        format!(
                            "Loaded \"{name}\". {missing} file(s) are missing — right-click > Locate file..."
                        )
                    } else {
                        format!("Loaded \"{name}\".")
                    };
                    load_tracks(paths, LoadMode::ReplaceAndPlay)
                }
            }
            Err(error) => {
                self.status = format!("Could not load playlist: {error}");
                Task::none()
            }
        }
    }

    pub(crate) fn delete_playlist(&mut self, name: String) {
        match library::delete(&config::playlist_dir(), &name) {
            Ok(()) => {
                if self.rename_target.as_deref() == Some(name.as_str()) {
                    self.rename_target = None;
                }
                self.status = format!("Deleted playlist \"{name}\".");
                self.refresh_playlists();
            }
            Err(error) => self.status = format!("Could not delete playlist: {error}"),
        }
    }

    pub(crate) fn confirm_rename(&mut self) {
        let Some(from) = self.rename_target.clone() else {
            return;
        };
        let to = self.rename_text.trim().to_string();
        if to.is_empty() {
            self.status = "The playlist needs a name.".to_string();
            return;
        }
        match library::rename(&config::playlist_dir(), &from, &to) {
            Ok(()) => {
                self.rename_target = None;
                self.status = format!("Renamed playlist to \"{to}\".");
                self.refresh_playlists();
            }
            Err(error) => self.status = format!("Could not rename playlist: {error}"),
        }
    }

    fn refresh_playlists(&mut self) {
        match library::list(&config::playlist_dir()) {
            Ok(entries) => self.playlists = entries,
            Err(error) => self.status = format!("Could not read playlists: {error}"),
        }
    }
}
