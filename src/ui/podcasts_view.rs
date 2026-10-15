// ------------------------------------------------------------
// Module: podcasts_view
// Purpose: Search, RSS add, favorites, and the episode list.
// ------------------------------------------------------------

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, text_input};
use iced::{Element, Length};

use crate::app::{MenuTarget, Message, PodcastMessage, State, Tab};
use crate::audio::Phase;
use crate::podcast::{self, Episode, KeyStatus, Show};
use crate::theme::{self, AppPalette};
use crate::ui::icons::{self, icon};
use crate::util::{format_date, format_date_span, format_time};

pub fn view(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let has_key =
        podcast::credentials(&state.podcasts.saved_key, &state.podcasts.saved_secret).is_some();
    row![sidebar(state, palette, has_key), episodes(state, palette),]
        .spacing(12)
        .height(Length::Fill)
        .into()
}

fn sidebar(state: &State, palette: AppPalette, has_key: bool) -> Element<'_, Message> {
    let mut body = column![text("Podcasts").size(20).color(palette.text)].spacing(10);
    if state.podcasts.offline {
        body = body.push(
            text("Offline - showing saved episodes")
                .size(13)
                .color(palette.muted),
        );
    }
    body = body.push(key_line(state, palette));
    if has_key {
        body = body.push(search_row(state, palette));
    } else {
        body = body.push(
            text("Add a Podcast Index key to search. An RSS URL still works.")
                .size(13)
                .color(palette.muted),
        );
    }
    body = body.push(rss_row(state, palette));
    body = body.push(text("Favorites").size(14).color(palette.text));
    body = body.push(scrollable(favorite_list(state, palette)).height(Length::Fill));
    if !state.podcasts.results.is_empty() {
        body = body.push(text("Search").size(14).color(palette.text));
        body = body.push(scrollable(result_list(state, palette)).height(160));
    }
    container(body)
        .padding(12)
        .width(320)
        .height(Length::Fill)
        .style(theme::surface_panel(palette))
        .into()
}

fn key_line(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let (label, color) = match &state.podcasts.key_status {
        KeyStatus::Valid => ("Podcast Index connected", palette.accent),
        KeyStatus::Missing => ("No API key", palette.muted),
        KeyStatus::Unchecked => ("API key saved, not checked yet", palette.muted),
        KeyStatus::Checking => ("Checking the API key...", palette.muted),
        KeyStatus::Rejected => ("API key rejected", palette.danger),
        KeyStatus::Unreachable(_) => ("Could not reach Podcast Index", palette.danger),
    };
    let label = text(label).size(13).color(color);
    if matches!(state.podcasts.key_status, KeyStatus::Valid | KeyStatus::Checking) {
        return label.into();
    }
    row![
        label,
        button(text("Open settings").size(12).color(palette.text))
            .padding([4, 8])
            .style(theme::quiet_button(palette))
            .on_press(Message::SetTab(Tab::Settings)),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn search_row(state: &State, palette: AppPalette) -> Element<'_, Message> {
    row![
        text_input("Search shows", &state.podcasts.query)
            .on_input(|value| Message::Podcast(PodcastMessage::QueryChanged(value)))
            .on_submit(Message::Podcast(PodcastMessage::Search))
            .padding(8)
            .width(Length::Fill),
        button(text("Search").size(13).color(palette.on_accent))
            .padding([8, 12])
            .style(theme::primary_button(palette))
            .on_press(Message::Podcast(PodcastMessage::Search)),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn rss_row(state: &State, palette: AppPalette) -> Element<'_, Message> {
    row![
        text_input("RSS feed URL", &state.podcasts.rss_input)
            .on_input(|value| Message::Podcast(PodcastMessage::RssChanged(value)))
            .on_submit(Message::Podcast(PodcastMessage::AddRss))
            .padding(8)
            .width(Length::Fill),
        button(text("Add").size(13).color(palette.on_accent))
            .padding([8, 12])
            .style(theme::primary_button(palette))
            .on_press(Message::Podcast(PodcastMessage::AddRss)),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn favorite_list(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let mut list = column![].spacing(4);
    if state.podcasts.favorites.is_empty() {
        list = list.push(
            text("Favorite a show to keep its episodes.")
                .size(13)
                .color(palette.muted),
        );
    }
    for show in &state.podcasts.favorites {
        list = list.push(show_row(show, palette));
    }
    list.into()
}

fn result_list(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let mut list = column![].spacing(4);
    for show in &state.podcasts.results {
        let saved = state
            .podcasts
            .favorites
            .iter()
            .any(|favorite| favorite.feed_url == show.feed_url);
        let message = if saved {
            Message::Podcast(PodcastMessage::OpenShow(show.feed_url.clone()))
        } else {
            Message::Podcast(PodcastMessage::RefreshShow(show.feed_url.clone()))
        };
        list = list.push(result_row(show, saved, message, palette));
    }
    list.into()
}

fn result_row(
    show: &Show,
    saved: bool,
    message: Message,
    palette: AppPalette,
) -> Element<'_, Message> {
    let label = if saved { "Open" } else { "Add" };
    row![
        text(&show.title)
            .size(14)
            .color(palette.text)
            .width(Length::Fill),
        button(text(label).size(12).color(palette.on_accent))
            .padding([6, 8])
            .style(theme::primary_button(palette))
            .on_press(message),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn show_row(show: &Show, palette: AppPalette) -> Element<'_, Message> {
    let title = text(&show.title)
        .size(14)
        .color(palette.text)
        .width(Length::Fill);
    mouse_area(
        button(title)
            .padding([6, 8])
            .width(Length::Fill)
            .style(theme::quiet_button(palette))
            .on_press(Message::Podcast(PodcastMessage::OpenShow(
                show.feed_url.clone(),
            ))),
    )
    .on_right_press(Message::OpenMenu(MenuTarget::Show(show.feed_url.clone())))
    .into()
}

fn episodes(state: &State, palette: AppPalette) -> Element<'_, Message> {
    let Some(show) = state.podcasts.selected_show() else {
        return container(
            text("Choose a favorite show to see its episodes.")
                .size(14)
                .color(palette.muted),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::surface_panel(palette))
        .into();
    };
    let phase = state.audio.snapshot().phase();
    let mut list = column![].spacing(4);
    for episode in &show.episodes {
        list = list.push(episode_row(
            state,
            &show.feed_url,
            episode,
            phase,
            palette,
        ));
    }
    let header = episode_header(state, show, palette);
    container(column![header, scrollable(list).height(Length::Fill)].spacing(10))
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::surface_panel(palette))
        .into()
}

fn episode_header<'a>(
    state: &'a State,
    show: &'a Show,
    palette: AppPalette,
) -> Element<'a, Message> {
    let subtitle = episode_subtitle(show);
    let mut heading = column![
        text(&show.title).size(20).color(palette.text),
        text(subtitle).size(13).color(palette.muted),
    ]
    .spacing(4)
    .width(Length::Fill);
    if !state.podcasts.older_loaded.contains(&show.feed_url) {
        heading = heading.push(
            text("The RSS feed lists only its latest episodes.")
                .size(12)
                .color(palette.muted),
        );
    }
    let mut header = row![heading].spacing(12).align_y(iced::Alignment::Center);
    if podcast::credentials(&state.podcasts.saved_key, &state.podcasts.saved_secret).is_some() {
        header = header.push(
            button(text("Load older episodes").size(13).color(palette.text))
                .padding([6, 10])
                .style(theme::quiet_button(palette))
                .on_press(Message::Podcast(PodcastMessage::LoadOlder(
                    show.feed_url.clone(),
                ))),
        );
    }
    header.into()
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
    let key = crate::podcast::PodcastState::progress_key(feed_url, &episode.guid);
    let (status, status_color) = if current {
        let label = match phase {
            Phase::Loading => "Loading...",
            Phase::Playing => "Playing",
            Phase::Paused => "Paused",
            Phase::Stopped => "Stopped",
        };
        (label.to_string(), palette.accent)
    } else if let Some(fraction) = state.podcasts.progress.get(&key) {
        (
            format!("{}%", (fraction * 100.0).round() as u8),
            palette.muted,
        )
    } else if episode.is_downloaded() {
        ("Downloaded".to_string(), palette.muted)
    } else {
        (String::new(), palette.muted)
    };
    let published = episode
        .published_secs
        .map(format_date)
        .unwrap_or_default();
    let title_color = if current { palette.accent } else { palette.text };
    let marker: Element<'a, Message> = if current {
        icon(&icons::NOTE, palette.accent, 14.0)
    } else {
        iced::widget::Space::new().width(14.0).height(14.0).into()
    };
    let details = row![
        marker,
        text(&episode.title)
            .size(14)
            .color(title_color)
            .width(Length::Fill),
        text(status).size(12).color(status_color),
        text(published).size(13).color(palette.muted).width(100),
        text(format_time(episode.duration()))
            .size(13)
            .color(palette.muted)
            .width(56),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);
    container(
        mouse_area(details)
            .on_double_click(Message::Podcast(PodcastMessage::PlayEpisode {
                feed_url: feed_url.to_string(),
                guid: episode.guid.clone(),
            }))
            .on_right_press(Message::OpenMenu(MenuTarget::Episode {
                feed_url: feed_url.to_string(),
                guid: episode.guid.clone(),
            })),
    )
    .padding([6, 8])
    .width(Length::Fill)
    .style(theme::episode_row(palette, current))
    .into()
}

// ------------------------------------------------------------
// Function: is_current_episode
// Purpose: True when this episode is the track loaded in the player,
//          whether it is streaming or playing a downloaded file.
// ------------------------------------------------------------
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
