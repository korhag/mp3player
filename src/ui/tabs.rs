// ------------------------------------------------------------
// Module: tabs
// Purpose: Library and Podcasts switch at the top of the window.
//          Playback stays visible on both.
// ------------------------------------------------------------

use iced::widget::{button, row, text};
use iced::{Element, Length};

use crate::app::{Message, Tab};
use crate::theme::{self, AppPalette};

pub fn view(selected: Tab, palette: AppPalette) -> Element<'static, Message> {
    row![
        tab("Library", Tab::Library, selected, palette),
        tab("Podcasts", Tab::Podcasts, selected, palette),
        tab("Settings", Tab::Settings, selected, palette),
    ]
    .spacing(8)
    .width(Length::Fill)
    .into()
}

fn tab(
    label: &'static str,
    tab: Tab,
    selected: Tab,
    palette: AppPalette,
) -> Element<'static, Message> {
    let color = if selected == tab {
        palette.on_accent
    } else {
        palette.text
    };
    let button = button(text(label).size(14).color(color))
        .padding([8, 14])
        .on_press(Message::SetTab(tab));
    if selected == tab {
        button.style(theme::primary_button(palette)).into()
    } else {
        button.style(theme::quiet_button(palette)).into()
    }
}
