// ------------------------------------------------------------
// Module: queue_view
// Purpose: The visible queue. Double-click a row to play it.
// ------------------------------------------------------------

use iced::widget::{button, column, container, mouse_area, row, scrollable, text};
use iced::{Element, Length};

use crate::app::{MenuTarget, Message};
use crate::library::Track;
use crate::theme::{self, AppPalette};
use crate::ui::icons::{self, icon};

// ------------------------------------------------------------
// Function: view
// Purpose: Queue header, empty state, and track rows.
// ------------------------------------------------------------
pub fn view<'a>(
    tracks: &'a [Track],
    current: Option<usize>,
    palette: AppPalette,
) -> Element<'a, Message> {
    let mut header = row![
        text("Queue").size(20).color(palette.text),
        text(format!("{} tracks", tracks.len()))
            .size(13)
            .color(palette.muted),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);
    header = header.push(iced::widget::Space::new().width(Length::Fill));
    header = header.push(text_button("Add files", Message::AddFiles, palette));
    header = header.push(text_button("Add folder", Message::AddFolder, palette));
    let mut clear = button(text("Clear").size(13).color(palette.text))
        .padding([6, 10])
        .style(theme::quiet_button(palette));
    if !tracks.is_empty() {
        clear = clear.on_press(Message::ClearQueue);
    }
    header = header.push(clear);

    let rows: Element<'a, Message> = if tracks.is_empty() {
        container(
            text("Drop audio files here, or use Add files. Double-click a track to play it.")
                .size(14)
                .color(palette.muted),
        )
        .padding(24)
        .into()
    } else {
        let mut list = column![].spacing(4);
        for (index, track) in tracks.iter().enumerate() {
            list = list.push(track_row(index, track, current == Some(index), palette));
        }
        list.into()
    };

    container(
        column![
            header,
            text("Double-click a track to play it. Right-click for more.")
                .size(12)
                .color(palette.muted),
            scrollable(rows).height(Length::Fill)
        ]
        .spacing(10),
    )
    .padding(12)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::surface_panel(palette))
    .into()
}

fn track_row<'a>(
    index: usize,
    track: &'a Track,
    current: bool,
    palette: AppPalette,
) -> Element<'a, Message> {
    let title_color = if track.missing {
        palette.danger
    } else if current {
        palette.accent
    } else {
        palette.text
    };
    let details = row![
        text(format!("{}", index + 1))
            .size(13)
            .color(palette.muted)
            .width(36),
        text(&track.title)
            .size(14)
            .color(title_color)
            .width(Length::Fill),
        text(track.artist_label())
            .size(13)
            .color(palette.muted)
            .width(200),
        text(track.duration_label())
            .size(13)
            .color(palette.muted)
            .width(56),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);

    let row = row![
        mouse_area(details)
            .on_double_click(Message::PlayIndex(index))
            .on_right_press(Message::OpenMenu(MenuTarget::Queue(index)))
            .interaction(iced::mouse::Interaction::Pointer),
        button(icon(&icons::TRASH, palette.muted, 14.0))
            .on_press(Message::RemoveIndex(index))
            .padding(6)
            .style(theme::danger_button(palette)),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    container(row)
        .padding([6, 8])
        .width(Length::Fill)
        .style(theme::track_row(palette, current))
        .into()
}

fn text_button<'a>(label: &'a str, message: Message, palette: AppPalette) -> Element<'a, Message> {
    button(text(label).size(13).color(palette.on_accent))
        .on_press(message)
        .padding([6, 10])
        .style(theme::primary_button(palette))
        .into()
}
