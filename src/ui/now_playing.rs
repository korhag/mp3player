// ------------------------------------------------------------
// Module: now_playing
// Purpose: Cover, title, and artist cluster in the bottom bar.
// ------------------------------------------------------------

use iced::widget::{column, container, image, row, text};
use iced::{Element, Length};

use crate::app::Message;
use crate::library::Track;
use crate::theme::AppPalette;
use crate::ui::icons::{self, icon};

// ------------------------------------------------------------
// Function: view
// Purpose: Left side of the transport bar.
// ------------------------------------------------------------
pub fn view<'a>(
    track: Option<&'a Track>,
    cover: Option<&'a image::Handle>,
    loading: bool,
    palette: AppPalette,
) -> Element<'a, Message> {
    let art: Element<'a, Message> = if let Some(cover) = cover {
        image(cover.clone())
            .width(64)
            .height(64)
            .content_fit(iced::ContentFit::Cover)
            .into()
    } else {
        container(icon(&icons::NOTE, palette.muted, 28.0))
            .width(64)
            .height(64)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .into()
    };

    let (title, artist) = match track {
        Some(track) => (track.title.as_str(), track.artist_label()),
        None => ("Nothing playing", String::new()),
    };
    let (subtitle, subtitle_color) = if loading {
        ("Loading stream...".to_string(), palette.accent)
    } else {
        (artist, palette.muted)
    };

    row![
        art,
        column![
            text(title).size(16).color(palette.text),
            text(subtitle).size(13).color(subtitle_color),
        ]
        .spacing(4),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill)
    .into()
}
