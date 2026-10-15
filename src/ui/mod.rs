// ------------------------------------------------------------
// Module: ui
// Purpose: Composes the minimal player layout.
// ------------------------------------------------------------

mod context_menu;
mod controls;
mod footer;
mod icons;
mod now_playing;
mod playlists_view;
mod podcasts_view;
mod queue_view;
mod settings_view;
mod tabs;

use iced::widget::{column, container, row, stack};
use iced::{Element, Length};

use crate::app::{Message, State, Tab};
use crate::theme;

// ------------------------------------------------------------
// Function: view
// Purpose: Sidebar, queue, transport bar, and footer.
// ------------------------------------------------------------
pub fn view(state: &State) -> Element<'_, Message> {
    let palette = state.theme_id.palette();
    let snapshot = state.audio.snapshot();
    let playing = snapshot.active && !snapshot.paused;

    let body = match state.tab {
        Tab::Library => row![
            playlists_view::view(
                &state.playlists,
                &state.playlist_name,
                state.rename_target.as_deref(),
                &state.rename_text,
                palette,
            ),
            queue_view::view(
                state.queue.tracks(),
                state.queue.current_display_index(),
                palette
            ),
        ]
        .spacing(12)
        .height(Length::Fill)
        .into(),
        Tab::Podcasts => podcasts_view::view(state, palette),
        Tab::Settings => settings_view::view(state, palette),
    };

    let page = column![
        tabs::view(state.tab, palette),
        body,
        controls::view(
            state.queue.current(),
            state.cover.as_ref(),
            state.position,
            state.seek_drag,
            state.volume,
            playing,
            snapshot.loading,
            state.queue.shuffle_enabled(),
            state.queue.repeat_mode(),
            palette,
        ),
        footer::view(&state.status, state.theme_id, palette),
    ]
    .spacing(12)
    .padding(16);

    let page = container(page)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::app_background(palette));
    if state.menu.is_some() {
        stack([page.into(), context_menu::layer(state, palette)]).into()
    } else {
        page.into()
    }
}
