// ------------------------------------------------------------
// Module: update_prompt
// Purpose: Card that offers to install a newer local build.
//          It sits over the player, the same way the context
//          menu does.
// ------------------------------------------------------------

use iced::widget::{button, column, container, opaque, row, text};
use iced::{Background, Color, Element, Length, Theme};

use crate::app::{Message, State};
use crate::theme::{self, AppPalette};
use crate::update;

// ------------------------------------------------------------
// Function: layer
// Purpose: Dims the player and centers the install card.
// ------------------------------------------------------------
pub fn layer<'a>(state: &'a State, palette: AppPalette) -> Element<'a, Message> {
    let Some(offer) = state.update_offer.as_ref() else {
        return column![].into();
    };
    let mut body = column![
        text("Install the latest version")
            .size(20)
            .color(palette.text),
        text(update::prompt_body(
            &offer.running_version,
            &offer.source_version,
        ))
        .size(14)
        .color(palette.text)
        .width(Length::Fill),
    ]
    .spacing(12)
    .width(Length::Fill);
    if !state.update_error.is_empty() {
        body = body.push(
            text(&state.update_error)
                .size(13)
                .color(palette.danger)
                .width(Length::Fill),
        );
    }
    body = body.push(
        row![
            button(text("Install").size(13).color(palette.on_accent))
                .padding([8, 12])
                .style(theme::primary_button(palette))
                .on_press(Message::InstallUpdate),
            button(text("Not now").size(13).color(palette.text))
                .padding([8, 12])
                .style(theme::quiet_button(palette))
                .on_press(Message::DismissUpdate),
        ]
        .spacing(8),
    );
    let card = container(body)
        .padding(20)
        .width(460)
        .style(theme::surface_panel(palette));
    opaque(
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .padding(24)
            .style(scrim),
    )
    .into()
}

fn scrim(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.55,
        })),
        ..Default::default()
    }
}
