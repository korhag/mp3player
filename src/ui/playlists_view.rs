// ------------------------------------------------------------
// Module: playlists_view
// Purpose: Sidebar for saving, loading, renaming, and deleting
//          M3U8 playlists.
// ------------------------------------------------------------

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::app::{MenuTarget, Message};
use crate::library::PlaylistEntry;
use crate::theme::{self, AppPalette};

// ------------------------------------------------------------
// Function: view
// Purpose: Playlist sidebar.
// ------------------------------------------------------------
pub fn view<'a>(
    playlists: &'a [PlaylistEntry],
    name_input: &'a str,
    rename_target: Option<&'a str>,
    rename_text: &'a str,
    palette: AppPalette,
) -> Element<'a, Message> {
    let mut list = column![].spacing(6);
    if playlists.is_empty() {
        list = list.push(
            text("Save the queue to keep a playlist.")
                .size(13)
                .color(palette.muted),
        );
    }
    for playlist in playlists {
        list = list.push(playlist_row(playlist, rename_target, rename_text, palette));
    }

    container(
        column![
            text("Playlists").size(20).color(palette.text),
            row![
                text_input("Playlist name", name_input)
                    .on_input(Message::PlaylistNameChanged)
                    .on_submit(Message::SavePlaylist)
                    .width(Length::Fill)
                    .padding(8),
                button(text("Save").size(13).color(palette.on_accent))
                    .on_press(Message::SavePlaylist)
                    .padding([8, 12])
                    .style(theme::primary_button(palette)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            scrollable(list).height(Length::Fill),
        ]
        .spacing(12),
    )
    .padding(12)
    .width(280)
    .height(Length::Fill)
    .style(theme::surface_panel(palette))
    .into()
}

fn playlist_row<'a>(
    playlist: &'a PlaylistEntry,
    rename_target: Option<&'a str>,
    rename_text: &'a str,
    palette: AppPalette,
) -> Element<'a, Message> {
    if rename_target == Some(playlist.name.as_str()) {
        return row![
            text_input("New name", rename_text)
                .on_input(Message::RenameInput)
                .on_submit(Message::ConfirmRename)
                .width(Length::Fill)
                .padding(6),
            small_button("OK", Message::ConfirmRename, palette, false),
            small_button("Cancel", Message::CancelRename, palette, false),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into();
    }

    mouse_area(
        button(text(&playlist.name).size(14).color(palette.text))
            .on_press(Message::LoadPlaylist(playlist.name.clone()))
            .padding([6, 8])
            .width(Length::Fill)
            .style(theme::quiet_button(palette)),
    )
    .on_right_press(Message::OpenMenu(MenuTarget::Playlist(
        playlist.name.clone(),
    )))
    .into()
}

fn small_button<'a>(
    label: &'a str,
    message: Message,
    palette: AppPalette,
    danger: bool,
) -> Element<'a, Message> {
    let color = if danger { palette.danger } else { palette.text };
    // The two style closures are different types, so each branch builds
    // its own button and erases it into an Element.
    if danger {
        button(text(label).size(12).color(color))
            .on_press(message)
            .padding([6, 8])
            .style(theme::danger_button(palette))
            .into()
    } else {
        button(text(label).size(12).color(color))
            .on_press(message)
            .padding([6, 8])
            .style(theme::quiet_button(palette))
            .into()
    }
}
