// ------------------------------------------------------------
// Module: podcast_panes
// Purpose: Favorites, search, and episodes as panes the user
//          can resize and drag by the title bar.
// ------------------------------------------------------------

use iced::widget::pane_grid::{Content, Controls, TitleBar};
use iced::widget::{button, pane_grid, text};
use iced::{Element, Length};

use crate::app::{Message, PodcastMessage, State};
use crate::podcast::PodcastPane;
use crate::theme::{self, AppPalette};
use crate::ui::podcasts_view;

// ------------------------------------------------------------
// Function: view
// Purpose: The podcast pane grid. Drag a title bar to dock a
//          pane on another edge. Drag a split to resize.
// ------------------------------------------------------------
pub fn view(state: &State, palette: AppPalette) -> Element<'_, Message> {
    pane_grid(&state.podcasts.panes, |_pane, kind, _maximized| {
        pane(state, *kind, palette)
    })
    .spacing(12)
    .width(Length::Fill)
    .height(Length::Fill)
    .on_drag(|event| Message::Podcast(PodcastMessage::PaneDragged(event)))
    .on_resize(8, |event| {
        Message::Podcast(PodcastMessage::PaneResized(event))
    })
    .into()
}

fn pane<'a>(state: &'a State, kind: PodcastPane, palette: AppPalette) -> Content<'a, Message> {
    let title = pane_title(state, kind);
    let mut bar = TitleBar::new(
        text(title)
            .size(14)
            .color(palette.text)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    )
    .padding([8, 12]);
    if kind == PodcastPane::Favorites {
        bar = bar
            .controls(Controls::new(reset_button(palette)))
            .always_show_controls();
    }
    let body = match kind {
        PodcastPane::Favorites => podcasts_view::favorites_pane(state, palette),
        PodcastPane::Search => podcasts_view::search_pane(state, palette),
        PodcastPane::Detail => podcasts_view::detail_pane(state, palette),
    };
    Content::new(body)
        .title_bar(bar)
        .style(theme::surface_panel(palette))
}

fn pane_title(state: &State, kind: PodcastPane) -> String {
    match kind {
        PodcastPane::Favorites => "Favorites".to_string(),
        PodcastPane::Search => "Search".to_string(),
        PodcastPane::Detail => state
            .podcasts
            .selected_show()
            .map(|show| clip_title(&show.title))
            .unwrap_or_else(|| "Episodes".to_string()),
    }
}

fn clip_title(title: &str) -> String {
    let mut chars = title.chars();
    let short: String = chars.by_ref().take(42).collect();
    if chars.next().is_some() {
        format!("{short}…")
    } else {
        short
    }
}

fn reset_button(palette: AppPalette) -> Element<'static, Message> {
    button(text("Reset").size(12).color(palette.text))
        .padding([4, 8])
        .style(theme::quiet_button(palette))
        .on_press(Message::Podcast(PodcastMessage::ResetLayout))
        .into()
}
