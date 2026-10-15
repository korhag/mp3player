// ------------------------------------------------------------
// Module: context_menu
// Purpose: A small menu drawn at the cursor. iced has no built-in
//          context menu, so this is a layer over the page.
// ------------------------------------------------------------

use iced::widget::{button, column, container, mouse_area, opaque, row, text};
use iced::{Element, Length};

use crate::app::{MenuTarget, Message, State};
use crate::theme::{self, AppPalette};

pub fn layer<'a>(state: &'a State, palette: AppPalette) -> Element<'a, Message> {
    let Some(cursor) = state.menu.as_ref().map(|menu| menu.cursor) else {
        return column![].into();
    };
    let panel = container(items(state, palette))
        .padding(6)
        .width(220)
        .style(theme::surface_panel(palette));
    mouse_area(
        container(
            column![
                iced::widget::Space::new().height(Length::Fixed(cursor.y.max(0.0))),
                row![
                    iced::widget::Space::new().width(Length::Fixed(cursor.x.max(0.0))),
                    opaque(panel),
                ],
            ]
            .width(Length::Shrink)
            .height(Length::Shrink),
        )
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .on_press(Message::CloseMenu)
    .into()
}

fn items(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let Some(menu) = &state.menu else {
        return column![].into();
    };
    let entries: Vec<(String, Message, bool)> = match &menu.target {
        MenuTarget::Queue(index) => queue_items(state, *index),
        MenuTarget::Playlist(name) => vec![
            (
                "Load".to_string(),
                Message::LoadPlaylist(name.clone()),
                false,
            ),
            (
                "Rename".to_string(),
                Message::StartRename(name.clone()),
                false,
            ),
            (
                "Delete".to_string(),
                Message::DeletePlaylist(name.clone()),
                true,
            ),
        ],
        MenuTarget::Episode { feed_url, guid } => episode_items(state, feed_url, guid),
        MenuTarget::Show(feed_url) => vec![
            (
                "Open".to_string(),
                Message::Podcast(crate::app::PodcastMessage::OpenShow(feed_url.clone())),
                false,
            ),
            (
                "Refresh episodes".to_string(),
                Message::Podcast(crate::app::PodcastMessage::RefreshShow(feed_url.clone())),
                false,
            ),
            (
                "Remove from favorites".to_string(),
                Message::Podcast(crate::app::PodcastMessage::RemoveShow(feed_url.clone())),
                true,
            ),
        ],
    };
    let mut column = column![].spacing(2);
    for (label, message, danger) in entries {
        column = column.push(entry(label, message, palette, danger));
    }
    column.into()
}

fn queue_items(state: &State, index: usize) -> Vec<(String, Message, bool)> {
    let mut entries = vec![
        ("Play".to_string(), Message::PlayIndex(index), false),
        ("Play next".to_string(), Message::PlayNext(index), false),
    ];
    let track = state.queue.tracks().get(index);
    let stream_only =
        track.is_some_and(|track| track.stream_url.is_some() && track.path.as_os_str().is_empty());
    if let Some(track) = track {
        if !track.path.as_os_str().is_empty() {
            entries.push((
                "Open file location".to_string(),
                Message::Reveal(track.path.clone()),
                false,
            ));
        }
    }
    if !stream_only {
        entries.push((
            "Locate file...".to_string(),
            Message::LocateTrack(index),
            false,
        ));
    }
    entries.push((
        "Remove from queue".to_string(),
        Message::RemoveIndex(index),
        true,
    ));
    entries
}

fn episode_items(state: &State, feed_url: &str, guid: &str) -> Vec<(String, Message, bool)> {
    let downloaded = state
        .podcasts
        .favorites
        .iter()
        .find(|show| show.feed_url == feed_url)
        .and_then(|show| show.episodes.iter().find(|episode| episode.guid == guid))
        .and_then(|episode| episode.local_path.clone().filter(|path| path.is_file()));
    let mut entries = vec![
        (
            "Play".to_string(),
            Message::Podcast(crate::app::PodcastMessage::PlayEpisode {
                feed_url: feed_url.to_string(),
                guid: guid.to_string(),
            }),
            false,
        ),
        (
            "Play next".to_string(),
            Message::Podcast(crate::app::PodcastMessage::QueueEpisodeNext {
                feed_url: feed_url.to_string(),
                guid: guid.to_string(),
            }),
            false,
        ),
    ];
    if downloaded.is_some() {
        entries.push((
            "Remove download".to_string(),
            Message::Podcast(crate::app::PodcastMessage::RemoveDownload {
                feed_url: feed_url.to_string(),
                guid: guid.to_string(),
            }),
            true,
        ));
    } else {
        entries.push((
            "Download".to_string(),
            Message::Podcast(crate::app::PodcastMessage::DownloadEpisode {
                feed_url: feed_url.to_string(),
                guid: guid.to_string(),
            }),
            false,
        ));
    }
    if let Some(path) = downloaded {
        entries.push((
            "Open file location".to_string(),
            Message::Reveal(path),
            false,
        ));
    }
    entries
}

fn entry<'a>(
    label: impl Into<String>,
    message: Message,
    palette: AppPalette,
    danger: bool,
) -> Element<'a, Message> {
    let color = if danger { palette.danger } else { palette.text };
    let button = button(text(label.into()).size(13).color(color))
        .padding([6, 8])
        .width(Length::Fill)
        .on_press(message);
    if danger {
        button.style(theme::danger_button(palette)).into()
    } else {
        button.style(theme::quiet_button(palette)).into()
    }
}
