// ------------------------------------------------------------
// Module: menu
// Purpose: Right-click targets and the actions that are not
//          playback or podcasts on their own.
// ------------------------------------------------------------

use std::path::PathBuf;

use iced::Point;
use iced::Task;

use crate::config;
use crate::system;

use super::{Message, State};

// ------------------------------------------------------------
// Enum: MenuTarget
// Purpose: Which row the open menu belongs to.
// ------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum MenuTarget {
    Queue(usize),
    Playlist(String),
    Episode { feed_url: String, guid: String },
    Show(String),
}

#[derive(Debug, Clone)]
pub struct MenuState {
    pub target: MenuTarget,
    pub cursor: Point,
}

impl State {
    pub(crate) fn open_menu(&mut self, target: MenuTarget) {
        self.menu = Some(MenuState {
            target,
            cursor: self.cursor,
        });
    }

    pub(crate) fn play_next_row(&mut self, index: usize) {
        self.menu = None;
        if !self.queue.play_next(index) {
            return;
        }
        if self.queue.current_display_index() == Some(index) {
            self.status = "This track is already playing.".to_string();
            return;
        }
        let streamed = self
            .queue
            .tracks()
            .get(index)
            .is_some_and(|track| !track.is_persistable());
        self.status = if streamed {
            "Will play next. Streamed episodes are skipped when the queue is saved.".to_string()
        } else {
            "Will play next.".to_string()
        };
        self.persist();
    }

    pub(crate) fn reveal(&mut self, path: PathBuf) {
        self.menu = None;
        if path.as_os_str().is_empty() || (!path.exists() && path.parent().is_none()) {
            self.status = "There is no file on disk to show.".to_string();
            return;
        }
        if let Err(error) = system::reveal(&path) {
            self.status = format!("Could not open the folder: {error}");
        }
    }

    pub(crate) fn locate_track(index: usize) -> Task<Message> {
        let extensions = config::EXTENSIONS.to_vec();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    rfd::FileDialog::new()
                        .add_filter("Audio", &extensions)
                        .pick_file()
                })
                .await
                .ok()
                .flatten()
            },
            move |path| Message::TrackLocated { index, path },
        )
    }
}
