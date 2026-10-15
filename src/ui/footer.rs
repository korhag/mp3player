// ------------------------------------------------------------
// Module: footer
// Purpose: Status line, application version, and theme picker.
// ------------------------------------------------------------

use iced::widget::{pick_list, row, text};
use iced::{Element, Length};

use crate::app::Message;
use crate::config;
use crate::theme::{AppPalette, ThemeId};

// ------------------------------------------------------------
// Function: view
// Purpose: Footer under the transport bar.
// ------------------------------------------------------------
pub fn view<'a>(status: &'a str, theme_id: ThemeId, palette: AppPalette) -> Element<'a, Message> {
    row![
        text(status)
            .size(12)
            .color(palette.muted)
            .width(Length::Fill),
        text(format!("v{}", config::VERSION))
            .size(12)
            .color(palette.muted),
        pick_list(ThemeId::ALL, Some(theme_id), Message::ThemeSelected).width(180),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .into()
}
