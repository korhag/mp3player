// ------------------------------------------------------------
// Module: app
// Purpose: Window state, messages, and which module handles each one.
// ------------------------------------------------------------

mod menu;
mod playback;
mod podcasts;

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use iced::widget::image;
use iced::window::{self, icon};
use iced::{Element, Point, Size, Subscription, Task, Theme};

use crate::audio::AudioHandle;
use crate::config;
use crate::library::{self, PlaylistEntry};
use crate::podcast::PodcastState;
use crate::queue::Queue;
use crate::settings::Settings;
use crate::theme::{self, ThemeId};
use crate::ui;

pub use menu::{MenuState, MenuTarget};
pub use podcasts::PodcastMessage;

static STARTUP: OnceLock<Vec<PathBuf>> = OnceLock::new();

// ------------------------------------------------------------
// Enum: LoadMode
// Purpose: What to do with tracks once metadata has been read.
// ------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum LoadMode {
    /// Append. `interrupt` starts the first new track immediately.
    Append {
        interrupt: bool,
    },
    ReplaceAndPlay,
    Restore {
        index: Option<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Library,
    Podcasts,
    Settings,
}

// ------------------------------------------------------------
// Enum: Message
// Purpose: Everything the interface can ask the player to do.
// ------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    TogglePlay,
    Next,
    Previous,
    SeekChanged(f32),
    SeekReleased,
    SeekBy(f32),
    VolumeChanged(f32),
    VolumeReleased,
    VolumeBy(f32),
    ToggleShuffle,
    CycleRepeat,
    PlayIndex(usize),
    RemoveIndex(usize),
    ClearQueue,
    AddFiles,
    AddFolder,
    PathsPicked(Option<Vec<PathBuf>>),
    TracksLoaded {
        tracks: Vec<crate::library::Track>,
        mode: LoadMode,
    },
    PlaylistNameChanged(String),
    SavePlaylist,
    LoadPlaylist(String),
    DeletePlaylist(String),
    StartRename(String),
    RenameInput(String),
    ConfirmRename,
    CancelRename,
    ThemeSelected(ThemeId),
    ExternalPaths(Vec<PathBuf>),
    FilesDropped(PathBuf),
    WindowClose,
    CursorMoved(Point),
    OpenMenu(MenuTarget),
    CloseMenu,
    Escape,
    PlayNext(usize),
    Reveal(PathBuf),
    LocateTrack(usize),
    TrackLocated {
        index: usize,
        path: Option<PathBuf>,
    },
    SetTab(Tab),
    Podcast(PodcastMessage),
}

// ------------------------------------------------------------
// Struct: State
// Purpose: The running player: queue, audio, sidebar, and podcasts.
// ------------------------------------------------------------
pub struct State {
    pub(crate) audio: AudioHandle,
    pub(crate) queue: Queue,
    pub(crate) playlists: Vec<PlaylistEntry>,
    pub(crate) playlist_name: String,
    pub(crate) rename_target: Option<String>,
    pub(crate) rename_text: String,
    pub(crate) theme_id: ThemeId,
    pub(crate) volume: f32,
    pub(crate) position: Duration,
    pub(crate) seek_drag: Option<f32>,
    pub(crate) cover: Option<image::Handle>,
    pub(crate) status: String,
    pub(crate) tab: Tab,
    pub(crate) cursor: Point,
    pub(crate) menu: Option<MenuState>,
    pub(crate) podcasts: PodcastState,
    seen_generation: u64,
    end_armed: bool,
}

// ------------------------------------------------------------
// Function: launch
// Purpose: Opens the window. `startup_paths` come from the
//          command line ("Open with") and replace the saved queue.
// ------------------------------------------------------------
pub fn launch(startup_paths: Vec<PathBuf>) -> iced::Result {
    let _ = STARTUP.set(startup_paths);
    iced::application(boot, update, view)
        .title(title)
        .subscription(subscription)
        .theme(theme)
        .window(window_settings())
        .run()
}

fn title(_state: &State) -> String {
    format!("{} {}", config::APP_NAME, config::VERSION)
}

fn theme(state: &State) -> Theme {
    theme::iced_theme(state.theme_id)
}

fn window_settings() -> window::Settings {
    window::Settings {
        size: Size::new(1040.0, 700.0),
        min_size: Some(Size::new(780.0, 520.0)),
        icon: window_icon(),
        ..Default::default()
    }
}

fn window_icon() -> Option<window::Icon> {
    let reader = ::image::ImageReader::new(std::io::Cursor::new(include_bytes!(
        "../../assets/icon.png"
    )))
    .with_guessed_format()
    .ok()?;
    let image = reader.decode().ok()?;
    let rgba = image.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    icon::from_rgba(rgba.into_raw(), width, height).ok()
}

fn boot() -> (State, Task<Message>) {
    let settings = Settings::load();
    let startup = STARTUP.get().cloned().unwrap_or_default();
    let state = State::from_settings(&settings);
    let mut tasks = Vec::new();
    if !startup.is_empty() {
        tasks.push(crate::tasks::load_tracks(
            startup,
            LoadMode::ReplaceAndPlay,
        ));
    } else if !settings.queue.is_empty() {
        tasks.push(crate::tasks::load_tracks(
            settings.queue.clone(),
            LoadMode::Restore {
                index: settings.queue_index,
            },
        ));
    }
    if crate::podcast::credentials(&settings.podcast_index_key, &settings.podcast_index_secret)
        .is_some()
    {
        tasks.push(Task::done(Message::Podcast(PodcastMessage::CheckKey)));
    }
    let task = if tasks.is_empty() {
        Task::none()
    } else {
        Task::batch(tasks)
    };
    (state, task)
}

fn view(state: &State) -> Element<'_, Message> {
    ui::view(state)
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Tick => state.on_tick(),
        Message::TogglePlay => state.toggle_play(),
        Message::Next => state.skip_next(false),
        Message::Previous => state.skip_previous(),
        Message::SeekChanged(value) => {
            state.seek_drag = Some(value);
            Task::none()
        }
        Message::SeekReleased => state.release_seek(),
        Message::SeekBy(delta) => state.seek_by(delta),
        Message::VolumeChanged(value) => {
            state.volume = value.clamp(0.0, 1.0);
            state.audio.set_volume(state.volume);
            Task::none()
        }
        Message::VolumeReleased => {
            state.persist();
            Task::none()
        }
        Message::VolumeBy(delta) => {
            state.volume = (state.volume + delta).clamp(0.0, 1.0);
            state.audio.set_volume(state.volume);
            state.persist();
            Task::none()
        }
        Message::ToggleShuffle => {
            state.queue.set_shuffle(!state.queue.shuffle_enabled());
            state.persist();
            Task::none()
        }
        Message::CycleRepeat => {
            state.queue.cycle_repeat();
            state.persist();
            Task::none()
        }
        Message::PlayIndex(index) => {
            state.menu = None;
            if state.queue.select_display(index) {
                state.start_current()
            } else {
                Task::none()
            }
        }
        Message::RemoveIndex(index) => state.remove_index(index),
        Message::ClearQueue => {
            state.menu = None;
            state.queue.clear();
            state.stop_playback();
            state.cover = None;
            state.persist();
            Task::none()
        }
        Message::AddFiles => crate::tasks::pick_audio(false),
        Message::AddFolder => crate::tasks::pick_audio(true),
        Message::PathsPicked(Some(paths)) => {
            crate::tasks::load_tracks(paths, LoadMode::Append { interrupt: false })
        }
        Message::PathsPicked(None) => Task::none(),
        Message::TracksLoaded { tracks, mode } => state.finish_load(tracks, mode),
        Message::PlaylistNameChanged(name) => {
            state.playlist_name = name;
            Task::none()
        }
        Message::SavePlaylist => {
            state.save_playlist();
            Task::none()
        }
        Message::LoadPlaylist(name) => {
            state.menu = None;
            state.load_playlist(name)
        }
        Message::DeletePlaylist(name) => {
            state.menu = None;
            state.delete_playlist(name);
            Task::none()
        }
        Message::StartRename(name) => {
            state.menu = None;
            state.rename_text = name.clone();
            state.rename_target = Some(name);
            Task::none()
        }
        Message::RenameInput(text) => {
            state.rename_text = text;
            Task::none()
        }
        Message::ConfirmRename => {
            state.confirm_rename();
            Task::none()
        }
        Message::CancelRename => {
            state.rename_target = None;
            Task::none()
        }
        Message::ThemeSelected(theme_id) => {
            state.theme_id = theme_id;
            state.persist();
            Task::none()
        }
        Message::ExternalPaths(paths) => {
            crate::tasks::load_tracks(paths, LoadMode::Append { interrupt: true })
        }
        Message::FilesDropped(path) => {
            crate::tasks::load_tracks(vec![path], LoadMode::Append { interrupt: false })
        }
        Message::WindowClose => {
            state.persist();
            Task::none()
        }
        Message::CursorMoved(position) => {
            state.cursor = position;
            Task::none()
        }
        Message::OpenMenu(target) => {
            state.open_menu(target);
            Task::none()
        }
        Message::CloseMenu => {
            state.menu = None;
            Task::none()
        }
        Message::Escape => {
            if state.menu.take().is_none() {
                state.rename_target = None;
            }
            Task::none()
        }
        Message::PlayNext(index) => {
            state.play_next_row(index);
            Task::none()
        }
        Message::Reveal(path) => {
            state.reveal(path);
            Task::none()
        }
        Message::LocateTrack(index) => {
            state.menu = None;
            State::locate_track(index)
        }
        Message::TrackLocated { index, path } => match path {
            Some(path) => state.relocate(index, path),
            None => Task::none(),
        },
        Message::SetTab(tab) => {
            state.menu = None;
            state.tab = tab;
            Task::none()
        }
        Message::Podcast(message) => state.podcast(message),
    }
}

fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::batch([
        iced::time::every(config::TICK).map(|_| Message::Tick),
        iced::event::listen_with(crate::tasks::shortcuts_and_drops),
        crate::tasks::ipc_subscription(),
    ])
}

impl State {
    fn from_settings(settings: &Settings) -> Self {
        let mut queue = Queue::new();
        queue.set_shuffle(settings.shuffle);
        queue.set_repeat(settings.repeat);
        Self {
            audio: AudioHandle::start(settings.volume),
            queue,
            playlists: library::list(&config::playlist_dir()).unwrap_or_default(),
            playlist_name: String::new(),
            rename_target: None,
            rename_text: String::new(),
            theme_id: settings.theme,
            volume: settings.volume.clamp(0.0, 1.0),
            position: Duration::ZERO,
            seek_drag: None,
            cover: None,
            status: String::new(),
            tab: Tab::Library,
            cursor: Point::ORIGIN,
            menu: None,
            podcasts: PodcastState::load(
                &settings.podcast_index_key,
                &settings.podcast_index_secret,
            ),
            seen_generation: 0,
            end_armed: false,
        }
    }

    pub(crate) fn persist(&self) {
        let (queue, queue_index) = self.queue.persistable_paths();
        let settings = Settings {
            theme: self.theme_id,
            volume: self.volume,
            shuffle: self.queue.shuffle_enabled(),
            repeat: self.queue.repeat_mode(),
            queue,
            queue_index,
            podcast_index_key: self.podcasts.saved_key.clone(),
            podcast_index_secret: self.podcasts.saved_secret.clone(),
        };
        let _ = settings.save();
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.persist();
    }
}
