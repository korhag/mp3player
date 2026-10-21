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
mod podcast_panes;
mod podcasts_view;
mod pointer;
mod queue_view;
mod settings_view;
mod tabs;

use iced::widget::scrollable::Scrollbar;
use iced::widget::{column, container, row, scrollable, stack};
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
        tabs::view(state.tab, state.theme_id, palette),
        body,
        controls::view(
            state.queue.current(),
            state.cover.as_ref(),
            state.position,
            state.seek_drag,
            state.volume,
            state.skip_back_secs,
            state.skip_forward_secs,
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
    let mut layers = vec![page.into()];
    if state
        .podcasts
        .drag
        .as_ref()
        .is_some_and(|gesture| gesture.armed)
    {
        layers.push(podcasts_view::drag_ghost(state, palette));
    }
    if state.menu.is_some() {
        layers.push(context_menu::layer(state, palette));
    }
    // The root stays a stack even when nothing floats over the page.
    // Swapping the page in as a plain container rebuilds every widget
    // under it, and the favorites list jumps back to the top.
    // The tracker sits outside the stack so the page tree never moves.
    pointer::tracker(stack(layers), state.podcasts.drag.is_some()).into()
}

// ------------------------------------------------------------
// Function: list_scroll
// Purpose: A vertical list whose scrollbar is reserved beside
//          the rows, so it cannot cover titles or buttons.
// ------------------------------------------------------------
pub fn list_scroll<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(content)
        .direction(scrollable::Direction::Vertical(Scrollbar::new().spacing(6)))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
