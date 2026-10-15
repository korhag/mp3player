// ------------------------------------------------------------
// Module: settings_view
// Purpose: The Settings tab. Podcast Index credentials live here
//          so saving a key is a deliberate step, not a hidden panel.
// ------------------------------------------------------------

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};

use crate::app::{Message, PodcastMessage, State};
use crate::podcast::{self, KeyStatus};
use crate::theme::{self, AppPalette};

pub fn view(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let dirty = state.podcasts.key_input != state.podcasts.saved_key
        || state.podcasts.secret_input != state.podcasts.saved_secret;
    let mut body = column![
        text("Podcast Index").size(20).color(palette.text),
        status_badge(&state.podcasts.key_status, palette),
        text_input("API key", &state.podcasts.key_input)
            .on_input(|value| Message::Podcast(PodcastMessage::KeyChanged(value)))
            .padding(8),
        text_input("API secret", &state.podcasts.secret_input)
            .secure(true)
            .on_input(|value| Message::Podcast(PodcastMessage::SecretChanged(value)))
            .padding(8),
    ]
    .spacing(10)
    .width(Length::Fill);
    if dirty {
        body = body.push(
            text("Unsaved changes")
                .size(12)
                .color(palette.muted),
        );
    }
    body = body.push(
        row![
            button(text("Save and check").size(13).color(palette.on_accent))
                .padding([8, 12])
                .style(theme::primary_button(palette))
                .on_press(Message::Podcast(PodcastMessage::SaveKeys)),
            button(text("Request a key").size(13).color(palette.text))
                .padding([8, 12])
                .style(theme::quiet_button(palette))
                .on_press(Message::Podcast(PodcastMessage::OpenSignup)),
        ]
        .spacing(8),
    );
    body = body.push(
        text("Request a key with your own email. Podcast Index emails the key and secret. Paste them here and press Save and check. Each person has their own key, and they turn away free inboxes such as Gmail.")
            .size(12)
            .color(palette.muted),
    );
    if podcast::using_environment() {
        body = body.push(
            text("Using PODCASTINDEX_API_KEY from the environment. That overrides the key saved here.")
                .size(12)
                .color(palette.muted),
        );
    }
    body = body.push(
        text("Stored in settings.json on this PC.")
            .size(12)
            .color(palette.muted),
    );
    let card = container(body)
        .padding(16)
        .width(520)
        .style(theme::surface_panel(palette));
    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn status_badge(status: &KeyStatus, palette: AppPalette) -> Element<'_, Message> {
    let (label, color, detail) = match status {
        KeyStatus::Missing => ("No API key", palette.muted, None),
        KeyStatus::Unchecked => ("Saved, not checked yet", palette.muted, None),
        KeyStatus::Checking => ("Checking the key...", palette.muted, None),
        KeyStatus::Valid => ("Connected", palette.accent, None),
        KeyStatus::Rejected => ("Key rejected by Podcast Index", palette.danger, None),
        KeyStatus::Unreachable(message) => (
            "Could not reach Podcast Index",
            palette.danger,
            Some(message.as_str()),
        ),
    };
    let mut column = column![
        row![
            text("●").size(16).color(color),
            text(label).size(14).color(palette.text),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
    ]
    .spacing(4);
    if let Some(detail) = detail {
        column = column.push(text(detail).size(12).color(palette.muted));
    }
    column.into()
}
