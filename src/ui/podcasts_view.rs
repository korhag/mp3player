// ------------------------------------------------------------
// Module: podcasts_view
// Purpose: Search, favorites, groups, and the episode list
//          drawn inside the podcast pane grid.
// ------------------------------------------------------------

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, mouse_area, row, text, text_input};
use iced::{Element, Length};

use crate::app::{MenuTarget, Message, PodcastMessage, State};
use crate::audio::Phase;
use crate::podcast::{
    self, DragItem, DropSlot, DropTarget, Episode, FavoriteItem, ListenState, Show,
};
use crate::theme::{self, AppPalette};
use crate::ui::icons::{self, icon};
use crate::util::{format_date, format_date_span, format_time};

// ------------------------------------------------------------
// Function: view
// Purpose: The Podcasts tab. Panes can be resized and dragged.
// ------------------------------------------------------------
pub fn view(state: &State, palette: AppPalette) -> Element<'_, Message> {
    super::podcast_panes::view(state, palette)
}

pub(super) fn favorites_pane(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let mut body = column![]
        .spacing(6)
        .height(Length::Fill)
        .width(Length::Fill);
    if state.podcasts.offline {
        body = body.push(
            text("Offline - showing saved episodes")
                .size(13)
                .color(palette.muted),
        );
    }
    let hint = if state.podcasts.favorites.is_empty() {
        "Favorite a show to keep its episodes."
    } else {
        "Drag shows onto each other to group them."
    };
    body = body.push(text(hint).size(12).color(palette.muted));
    body = body.push(super::list_scroll(favorite_list(state, palette)));
    container(body)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(super) fn search_pane(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let mut body = column![key_line(state, palette), search_row(state, palette)]
        .spacing(6)
        .height(Length::Fill)
        .width(Length::Fill);
    if state.podcasts.results.is_empty() {
        body = body.push(
            text("Search or paste an RSS URL above.")
                .size(13)
                .color(palette.muted),
        );
    } else {
        body = body.push(super::list_scroll(result_list(state, palette)));
    }
    container(body)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(super) fn detail_pane(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let Some(show) = state.podcasts.selected_show() else {
        return container(
            text("Choose a show to see its episodes.")
                .size(14)
                .color(palette.muted),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    };
    let phase = state.audio.snapshot().phase();
    let filter = state.podcasts.episode_filter.trim().to_lowercase();
    let mut list = column![].spacing(2);
    let mut shown = 0usize;
    for episode in &show.episodes {
        if !filter.is_empty() && !episode.title.to_lowercase().contains(&filter) {
            continue;
        }
        shown += 1;
        list = list.push(episode_row(state, &show.feed_url, episode, phase, palette));
    }
    if shown == 0 {
        let message = if filter.is_empty() {
            "This show has no episodes yet."
        } else {
            "No episodes match that filter."
        };
        list = list.push(text(message).size(13).color(palette.muted));
    }
    let mut body = column![detail_switch(state.podcasts.episodes_only, palette)]
        .spacing(6)
        .height(Length::Fill)
        .width(Length::Fill);
    if !state.podcasts.episodes_only {
        body = body.push(show_summary(state, show, palette));
    }
    body = body.push(episode_tools(state, show, palette));
    body = body.push(super::list_scroll(list));
    container(body)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ------------------------------------------------------------
// Function: drag_ghost
// Purpose: A label that follows the pointer while a favorite
//          or a group is being dragged. It does not take clicks.
// ------------------------------------------------------------
pub fn drag_ghost(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let Some(gesture) = state.podcasts.drag.as_ref() else {
        return column![].into();
    };
    let label = match &gesture.item {
        DragItem::Show(feed_url) => state
            .podcasts
            .favorites
            .iter()
            .find(|show| &show.feed_url == feed_url)
            .map(|show| show.title.clone())
            .unwrap_or_else(|| "Show".to_string()),
        DragItem::Group(index) => match state.podcasts.groups.get(*index) {
            Some(FavoriteItem::Group { name, .. }) => name.clone(),
            _ => "Group".to_string(),
        },
    };
    let panel = container(text(label).size(13).color(palette.text))
        .padding([6, 10])
        .style(theme::surface_panel(palette));
    container(
        column![
            iced::widget::Space::new().height(Length::Fixed((state.cursor.y + 14.0).max(0.0))),
            row![
                iced::widget::Space::new().width(Length::Fixed((state.cursor.x + 14.0).max(0.0))),
                panel,
            ],
        ]
        .width(Length::Shrink)
        .height(Length::Shrink),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn key_line(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let (label, color) = match &state.podcasts.key_status {
        podcast::KeyStatus::Valid => ("Podcast Index connected", palette.accent),
        podcast::KeyStatus::Missing => ("No API key", palette.muted),
        podcast::KeyStatus::Unchecked => ("API key saved, not checked yet", palette.muted),
        podcast::KeyStatus::Checking => ("Checking the API key...", palette.muted),
        podcast::KeyStatus::Rejected => ("API key rejected", palette.danger),
        podcast::KeyStatus::Unreachable(_) => ("Could not reach Podcast Index", palette.danger),
    };
    let label = text(label).size(13).color(color);
    if matches!(
        state.podcasts.key_status,
        podcast::KeyStatus::Valid | podcast::KeyStatus::Checking
    ) {
        return label.into();
    }
    row![
        label,
        button(text("Open settings").size(12).color(palette.text))
            .padding([4, 8])
            .style(theme::quiet_button(palette))
            .on_press(Message::SetTab(crate::app::Tab::Settings)),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn search_row(state: &State, palette: AppPalette) -> Element<'_, Message> {
    row![
        text_input("Search shows or paste an RSS URL", &state.podcasts.query)
            .on_input(|value| Message::Podcast(PodcastMessage::QueryChanged(value)))
            .on_submit(Message::Podcast(PodcastMessage::Search))
            .padding(6)
            .width(Length::Fill),
        button(text("Search").size(12).color(palette.on_accent))
            .padding([6, 10])
            .style(theme::primary_button(palette))
            .on_press(Message::Podcast(PodcastMessage::Search)),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn favorite_list(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let dragging_show = state
        .podcasts
        .drag
        .as_ref()
        .is_some_and(|gesture| gesture.armed && matches!(gesture.item, DragItem::Show(_)));
    let dragging = state
        .podcasts
        .drag
        .as_ref()
        .is_some_and(|gesture| gesture.armed);
    let mut list = column![].spacing(2);
    for (index, item) in state.podcasts.groups.iter().enumerate() {
        if dragging {
            list = list.push(insertion_slot(state, DropSlot::Before(index), palette));
        }
        match item {
            FavoriteItem::Show(feed_url) => {
                if let Some(show) = find_favorite(state, feed_url) {
                    list = list.push(favorite_show_row(state, show, palette));
                }
            }
            FavoriteItem::Group {
                name,
                collapsed,
                shows,
            } => {
                list = list.push(group_header(
                    state,
                    index,
                    name,
                    *collapsed,
                    shows.len(),
                    palette,
                ));
                if *collapsed {
                    continue;
                }
                let mut inner = column![].spacing(2).width(Length::Fill);
                for (inner_index, feed_url) in shows.iter().enumerate() {
                    if dragging_show {
                        inner = inner.push(insertion_slot(
                            state,
                            DropSlot::InGroup {
                                group: index,
                                before: inner_index,
                            },
                            palette,
                        ));
                    }
                    if let Some(show) = find_favorite(state, feed_url) {
                        inner = inner.push(favorite_show_row(state, show, palette));
                    }
                }
                if dragging_show {
                    inner = inner.push(insertion_slot(
                        state,
                        DropSlot::InGroup {
                            group: index,
                            before: shows.len(),
                        },
                        palette,
                    ));
                }
                list = list.push(expanded_group(inner, palette));
            }
        }
    }
    if dragging {
        list = list.push(insertion_slot(
            state,
            DropSlot::Before(state.podcasts.groups.len()),
            palette,
        ));
    }
    list.into()
}

fn find_favorite<'a>(state: &'a State, feed_url: &str) -> Option<&'a Show> {
    state
        .podcasts
        .favorites
        .iter()
        .find(|show| show.feed_url == feed_url)
}

fn insertion_slot(state: &State, slot: DropSlot, palette: AppPalette) -> Element<'_, Message> {
    let target = DropTarget::Slot(slot);
    let active = state.podcasts.drop_target.as_ref() == Some(&target);
    mouse_area(
        container(iced::widget::Space::new().width(Length::Fill).height(6))
            .width(Length::Fill)
            .height(8)
            .style(theme::drop_slot(palette, active)),
    )
    .on_enter(Message::Podcast(PodcastMessage::DragTarget(target.clone())))
    .on_exit(Message::Podcast(PodcastMessage::DragLeave(target)))
    .into()
}

// ------------------------------------------------------------
// Function: expanded_group
// Purpose: Shows in an open group sit beside a vertical rail so
//          the group reads as one expanded block.
// ------------------------------------------------------------
fn expanded_group<'a>(
    children: iced::widget::Column<'a, Message>,
    palette: AppPalette,
) -> Element<'a, Message> {
    let rail = container(iced::widget::Space::new())
        .width(2)
        .height(Length::Fill)
        .style(theme::group_rail(palette));
    row![rail, children]
        .spacing(8)
        .padding(iced::Padding::ZERO.top(2).bottom(2).left(12))
        .width(Length::Fill)
        .into()
}

fn group_header<'a>(
    state: &'a State,
    index: usize,
    name: &'a str,
    collapsed: bool,
    count: usize,
    palette: AppPalette,
) -> Element<'a, Message> {
    if state.podcasts.rename_group == Some(index) {
        return row![
            text_input("Group name", &state.podcasts.group_rename_text)
                .on_input(|value| Message::Podcast(PodcastMessage::GroupRenameInput(value)))
                .on_submit(Message::Podcast(PodcastMessage::ConfirmGroupRename))
                .width(Length::Fill)
                .padding(6),
            button(text("OK").size(12).color(palette.text))
                .padding([6, 8])
                .style(theme::quiet_button(palette))
                .on_press(Message::Podcast(PodcastMessage::ConfirmGroupRename)),
            button(text("Cancel").size(12).color(palette.text))
                .padding([6, 8])
                .style(theme::quiet_button(palette))
                .on_press(Message::Podcast(PodcastMessage::CancelGroupRename)),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .into();
    }
    let target = DropTarget::OnGroup(index);
    let hovered = state.podcasts.drop_target.as_ref() == Some(&target);
    let mark = if collapsed { "▸" } else { "▾" };
    let body = row![
        button(
            text(mark)
                .size(18)
                .width(16)
                .align_x(iced::Alignment::Center)
                .color(palette.muted),
        )
        .padding([1, 2])
        .style(theme::quiet_button(palette))
        .on_press(Message::Podcast(PodcastMessage::ToggleGroup(index))),
        text(name).size(14).color(palette.text),
        text(format!(
            "{count} {}",
            if count == 1 { "show" } else { "shows" }
        ))
        .size(12)
        .color(palette.muted),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);
    mouse_area(
        container(body)
            .padding([2, 4])
            .width(Length::Fill)
            .style(theme::episode_row(palette, false, hovered)),
    )
    .on_press(Message::Podcast(PodcastMessage::DragPressed {
        item: DragItem::Group(index),
        x: state.cursor.x,
        y: state.cursor.y,
    }))
    .on_double_click(Message::Podcast(PodcastMessage::StartGroupRename(index)))
    .on_right_press(Message::OpenMenu(MenuTarget::Group(index)))
    .on_enter(Message::Podcast(PodcastMessage::DragTarget(target.clone())))
    .on_exit(Message::Podcast(PodcastMessage::DragLeave(target)))
    .interaction(iced::mouse::Interaction::Grab)
    .into()
}

fn favorite_show_row<'a>(
    state: &'a State,
    show: &'a Show,
    palette: AppPalette,
) -> Element<'a, Message> {
    let selected = state.podcasts.selected.as_deref() == Some(show.feed_url.as_str());
    let target = DropTarget::OnShow(show.feed_url.clone());
    let hovered = state.podcasts.drop_target.as_ref() == Some(&target);
    let mut lines = column![wrapped_title(
        &show.title,
        13.0,
        if selected {
            palette.accent
        } else {
            palette.text
        }
    )]
    .spacing(0)
    .width(Length::Fill);
    if !show.author.is_empty() {
        lines = lines.push(text(&show.author).size(12).color(palette.muted));
    }
    mouse_area(
        container(lines)
            .padding([2, 4])
            .width(Length::Fill)
            .style(theme::episode_row(palette, selected, hovered)),
    )
    .on_press(Message::Podcast(PodcastMessage::DragPressed {
        item: DragItem::Show(show.feed_url.clone()),
        x: state.cursor.x,
        y: state.cursor.y,
    }))
    .on_right_press(Message::OpenMenu(MenuTarget::Show(show.feed_url.clone())))
    .on_enter(Message::Podcast(PodcastMessage::DragTarget(target.clone())))
    .on_exit(Message::Podcast(PodcastMessage::DragLeave(target)))
    .interaction(iced::mouse::Interaction::Grab)
    .into()
}

fn result_list(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let mut list = column![].spacing(2);
    for show in &state.podcasts.results {
        let selected = state.podcasts.selected.as_deref() == Some(show.feed_url.as_str());
        list = list.push(result_row(
            show,
            state.podcasts.is_favorite(&show.feed_url),
            selected,
            palette,
        ));
    }
    list.into()
}

fn result_row(
    show: &Show,
    saved: bool,
    selected: bool,
    palette: AppPalette,
) -> Element<'_, Message> {
    let marker = if saved { "Saved" } else { "Preview" };
    let marker_color = if saved { palette.accent } else { palette.muted };
    let title_color = if selected {
        palette.accent
    } else {
        palette.text
    };
    let mut lines = column![wrapped_title(&show.title, 13.0, title_color)]
        .spacing(1)
        .width(Length::Fill);
    let mut meta = row![text(marker).size(12).color(marker_color)].spacing(8);
    if !show.author.is_empty() {
        meta = meta.push(text(&show.author).size(12).color(palette.muted));
    }
    lines = lines.push(meta);
    let star = if saved {
        &icons::STAR_FILLED
    } else {
        &icons::STAR
    };
    let star_color = if saved { palette.accent } else { palette.muted };
    let row = row![
        mouse_area(lines)
            .on_press(Message::Podcast(PodcastMessage::PreviewShow(
                show.feed_url.clone(),
            )))
            .interaction(iced::mouse::Interaction::Pointer),
        button(icon(star, star_color, 14.0))
            .padding(2)
            .style(theme::quiet_button(palette))
            .on_press(Message::Podcast(PodcastMessage::ToggleFavorite(
                show.feed_url.clone(),
            ))),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);
    container(row)
        .padding([2, 6])
        .width(Length::Fill)
        .style(theme::episode_row(palette, false, selected))
        .into()
}

fn detail_switch(episodes_only: bool, palette: AppPalette) -> Element<'static, Message> {
    row![
        detail_tab("Show", false, episodes_only, palette),
        detail_tab("Episodes", true, episodes_only, palette),
    ]
    .spacing(6)
    .into()
}

fn detail_tab(
    label: &'static str,
    episodes_only: bool,
    selected_episodes_only: bool,
    palette: AppPalette,
) -> Element<'static, Message> {
    let selected = episodes_only == selected_episodes_only;
    let color = if selected {
        palette.on_accent
    } else {
        palette.text
    };
    let button = button(text(label).size(12).color(color))
        .padding([4, 10])
        .on_press(Message::Podcast(PodcastMessage::SetEpisodesOnly(
            episodes_only,
        )));
    if selected {
        button.style(theme::primary_button(palette)).into()
    } else {
        button.style(theme::quiet_button(palette)).into()
    }
}

fn show_summary<'a>(state: &'a State, show: &'a Show, palette: AppPalette) -> Element<'a, Message> {
    let favorite = state.podcasts.is_favorite(&show.feed_url);
    let art = artwork(state, show, palette);
    let mut heading = column![
        text(&show.title)
            .size(18)
            .color(palette.text)
            .wrapping(Wrapping::WordOrGlyph),
        text(episode_subtitle(show)).size(12).color(palette.muted),
    ]
    .spacing(2)
    .width(Length::Fill);
    if !show.description.is_empty() {
        heading = heading.push(
            text(brief(&show.description))
                .size(12)
                .color(palette.muted)
                .wrapping(Wrapping::WordOrGlyph),
        );
    }
    heading = heading.push(favorite_button(show, favorite, palette));
    if !favorite {
        heading = heading.push(
            text("Favorite this show to download episodes and load older ones.")
                .size(12)
                .color(palette.muted),
        );
    } else if !state.podcasts.older_loaded.contains(&show.feed_url) {
        heading = heading.push(
            text("The RSS feed lists only its latest episodes.")
                .size(12)
                .color(palette.muted),
        );
    }
    row![art, heading]
        .spacing(10)
        .align_y(iced::Alignment::Start)
        .into()
}

fn episode_tools<'a>(
    state: &'a State,
    show: &'a Show,
    palette: AppPalette,
) -> Element<'a, Message> {
    let favorite = state.podcasts.is_favorite(&show.feed_url);
    let has_key =
        podcast::credentials(&state.podcasts.saved_key, &state.podcasts.saved_secret).is_some();
    let mut tools = row![
        text_input("Filter episodes", &state.podcasts.episode_filter)
            .on_input(|value| Message::Podcast(PodcastMessage::EpisodeFilterChanged(value)))
            .padding(6)
            .width(Length::Fill),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);
    if favorite && has_key {
        tools = tools.push(
            button(text("Load older episodes").size(12).color(palette.text))
                .padding([4, 8])
                .style(theme::quiet_button(palette))
                .on_press(Message::Podcast(PodcastMessage::LoadOlder(
                    show.feed_url.clone(),
                ))),
        );
    }
    tools.into()
}

fn favorite_button(show: &Show, favorite: bool, palette: AppPalette) -> Element<'_, Message> {
    let message = Message::Podcast(PodcastMessage::ToggleFavorite(show.feed_url.clone()));
    if favorite {
        button(text("Remove from favorites").size(12).color(palette.danger))
            .padding([4, 8])
            .style(theme::danger_button(palette))
            .on_press(message)
            .into()
    } else {
        button(text("Add to favorites").size(12).color(palette.on_accent))
            .padding([4, 8])
            .style(theme::primary_button(palette))
            .on_press(message)
            .into()
    }
}

fn artwork<'a>(state: &'a State, show: &'a Show, palette: AppPalette) -> Element<'a, Message> {
    let handle = show
        .image_url
        .as_ref()
        .and_then(|url| state.artwork.get(url));
    if let Some(handle) = handle {
        iced::widget::image(handle.clone())
            .width(52)
            .height(52)
            .content_fit(iced::ContentFit::Cover)
            .into()
    } else {
        container(icon(&icons::NOTE, palette.muted, 22.0))
            .width(52)
            .height(52)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
            .into()
    }
}

fn episode_subtitle(show: &Show) -> String {
    let count = format!("{} episodes", show.episodes.len());
    let dates: Vec<i64> = show
        .episodes
        .iter()
        .filter_map(|episode| episode.published_secs)
        .collect();
    let span = dates
        .iter()
        .copied()
        .min()
        .zip(dates.iter().copied().max())
        .map(|(oldest, newest)| format_date_span(oldest, newest));
    match (show.author.is_empty(), span) {
        (false, Some(span)) => format!("{} · {} · {}", show.author, count, span),
        (true, Some(span)) => format!("{count} · {span}"),
        (false, None) => format!("{} · {count}", show.author),
        (true, None) => count,
    }
}

fn episode_row<'a>(
    state: &'a State,
    feed_url: &'a str,
    episode: &'a Episode,
    phase: Phase,
    palette: AppPalette,
) -> Element<'a, Message> {
    let current = is_current_episode(state, episode);
    let selected = state
        .podcasts
        .selected_episode
        .as_ref()
        .is_some_and(|(selected_feed, guid)| selected_feed == feed_url && guid == &episode.guid);
    let key = podcast::PodcastState::progress_key(feed_url, &episode.guid);
    let download = state.podcasts.progress.get(&key).copied();
    let mark = episode_mark(state, episode, current);
    let published = episode.published_secs.map(format_date).unwrap_or_default();
    let title_color = if current {
        palette.accent
    } else {
        palette.text
    };
    let play_icon = if current && phase == Phase::Playing {
        &icons::PAUSE
    } else {
        &icons::PLAY
    };
    let play_message = if current && matches!(phase, Phase::Playing | Phase::Paused) {
        Message::TogglePlay
    } else {
        Message::Podcast(PodcastMessage::PlayEpisode {
            feed_url: feed_url.to_string(),
            guid: episode.guid.clone(),
            from_start: false,
        })
    };
    let mut details = row![wrapped_title(&episode.title, 13.0, title_color)]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);
    if let Some((note, note_color)) = episode_note(state, episode, current, phase, mark, palette) {
        details = details.push(text(note).size(12).color(note_color));
    }
    details = details.push(text(published).size(12).color(palette.muted).width(92));
    details = details.push(
        text(format_time(episode.duration()))
            .size(12)
            .color(palette.muted)
            .width(52),
    );
    let row = row![
        button(icon(play_icon, palette.text, 13.0))
            .padding(2)
            .style(theme::quiet_button(palette))
            .on_press(play_message),
        listen_mark(feed_url, episode, mark, palette),
        mouse_area(details)
            .on_press(Message::Podcast(PodcastMessage::SelectEpisode {
                feed_url: feed_url.to_string(),
                guid: episode.guid.clone(),
            }))
            .on_double_click(Message::Podcast(PodcastMessage::PlayEpisode {
                feed_url: feed_url.to_string(),
                guid: episode.guid.clone(),
                from_start: false,
            }))
            .on_right_press(Message::OpenMenu(MenuTarget::Episode {
                feed_url: feed_url.to_string(),
                guid: episode.guid.clone(),
            }))
            .interaction(iced::mouse::Interaction::Pointer),
        download_button(feed_url, episode, download, palette),
    ]
    .spacing(2)
    .align_y(iced::Alignment::Center);
    container(row)
        .padding([1, 6])
        .width(Length::Fill)
        .style(theme::episode_row(palette, current, selected))
        .into()
}

fn episode_mark(state: &State, episode: &Episode, current: bool) -> ListenState {
    let saved = podcast::listen_state(&state.podcasts.listening, &episode.audio_url);
    if saved == ListenState::Finished {
        return ListenState::Finished;
    }
    if current && state.position.as_secs_f32() >= 1.0 {
        return ListenState::Started;
    }
    saved
}

fn listen_mark<'a>(
    feed_url: &'a str,
    episode: &'a Episode,
    mark: ListenState,
    palette: AppPalette,
) -> Element<'a, Message> {
    let (glyph, color) = match mark {
        ListenState::Finished => ("✓", palette.accent),
        ListenState::Started => ("●", palette.accent),
        ListenState::New => ("○", palette.muted),
    };
    button(
        text(glyph)
            .size(13)
            .width(14)
            .align_x(iced::Alignment::Center)
            .color(color),
    )
    .padding([1, 2])
    .style(theme::quiet_button(palette))
    .on_press(Message::Podcast(PodcastMessage::MarkPlayed {
        feed_url: feed_url.to_string(),
        guid: episode.guid.clone(),
        played: mark != ListenState::Finished,
    }))
    .into()
}

fn episode_note(
    state: &State,
    episode: &Episode,
    current: bool,
    phase: Phase,
    mark: ListenState,
    palette: AppPalette,
) -> Option<(String, iced::Color)> {
    if current {
        let label = match phase {
            Phase::Loading => "Loading...",
            Phase::Playing => "Playing",
            Phase::Paused => "Paused",
            Phase::Stopped => "Stopped",
        };
        return Some((label.to_string(), palette.accent));
    }
    if mark == ListenState::Started
        && let Some(saved) = state.podcasts.listening.positions.get(&episode.audio_url)
    {
        return Some((
            remaining_label(episode.duration_secs, *saved),
            palette.muted,
        ));
    }
    None
}

fn remaining_label(duration_secs: u64, saved_secs: f32) -> String {
    if duration_secs == 0 {
        return format!(
            "at {}",
            format_time(std::time::Duration::from_secs_f32(saved_secs))
        );
    }
    let left = duration_secs.saturating_sub(saved_secs.round().max(0.0) as u64);
    format!("{} left", format_time(std::time::Duration::from_secs(left)))
}

fn download_button<'a>(
    feed_url: &'a str,
    episode: &'a Episode,
    download: Option<f32>,
    palette: AppPalette,
) -> Element<'a, Message> {
    if let Some(fraction) = download {
        return text(format!("{}%", (fraction * 100.0).round() as u8))
            .size(12)
            .color(palette.muted)
            .into();
    }
    if episode.is_downloaded() {
        return text("Saved").size(12).color(palette.accent).into();
    }
    button(text("↓").size(13).color(palette.text))
        .padding([2, 6])
        .style(theme::quiet_button(palette))
        .on_press(Message::Podcast(PodcastMessage::DownloadEpisode {
            feed_url: feed_url.to_string(),
            guid: episode.guid.clone(),
        }))
        .into()
}

fn wrapped_title(label: &str, size: f32, color: iced::Color) -> Element<'_, Message> {
    text(label.to_string())
        .size(size)
        .color(color)
        .wrapping(Wrapping::WordOrGlyph)
        .width(Length::Fill)
        .into()
}

fn brief(value: &str) -> String {
    let mut chars = value.chars();
    let short: String = chars.by_ref().take(280).collect();
    if chars.next().is_some() {
        format!("{short}…")
    } else {
        short
    }
}

fn is_current_episode(state: &State, episode: &Episode) -> bool {
    let Some(track) = state.queue.current() else {
        return false;
    };
    if track.stream_url.as_deref() == Some(episode.audio_url.as_str()) {
        return true;
    }
    episode
        .local_path
        .as_ref()
        .is_some_and(|path| !path.as_os_str().is_empty() && path == &track.path)
}
