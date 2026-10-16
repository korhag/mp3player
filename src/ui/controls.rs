// ------------------------------------------------------------
// Module: controls
// Purpose: Transport buttons, seek bar, and volume slider.
// ------------------------------------------------------------

use std::time::Duration;

use iced::widget::{button, column, container, row, slider, text};
use iced::{Element, Length};

use crate::app::Message;
use crate::config;
use crate::library::Track;
use crate::queue::RepeatMode;
use crate::theme::{self, AppPalette};
use crate::ui::icons::{self, icon};
use crate::ui::now_playing;
use crate::util::format_time;

// ------------------------------------------------------------
// Function: view
// Purpose: Bottom playback bar.
// ------------------------------------------------------------
pub fn view<'a>(
    track: Option<&'a Track>,
    cover: Option<&'a iced::widget::image::Handle>,
    position: Duration,
    seek_drag: Option<f32>,
    volume: f32,
    skip_back_secs: u32,
    skip_forward_secs: u32,
    playing: bool,
    loading: bool,
    shuffle: bool,
    repeat: RepeatMode,
    palette: AppPalette,
) -> Element<'a, Message> {
    let icon_color = palette.text;
    let play_icon = if playing {
        icon(&icons::PAUSE, icon_color, 18.0)
    } else {
        icon(&icons::PLAY, icon_color, 18.0)
    };
    let repeat_icon = match repeat {
        RepeatMode::One => icon(&icons::REPEAT_ONE, palette.accent, 16.0),
        RepeatMode::All => icon(&icons::REPEAT, palette.accent, 16.0),
        RepeatMode::Off => icon(&icons::REPEAT, palette.muted, 16.0),
    };
    let shuffle_color = if shuffle {
        palette.accent
    } else {
        palette.muted
    };

    let transport = row![
        icon_button(
            icon(&icons::PREVIOUS, icon_color, 16.0),
            Message::Previous,
            palette,
            false
        ),
        skip_button(&icons::REWIND, skip_back_secs, Message::SkipBack, palette,),
        icon_button(play_icon, Message::TogglePlay, palette, false),
        skip_button(
            &icons::FORWARD,
            skip_forward_secs,
            Message::SkipForward,
            palette,
        ),
        icon_button(
            icon(&icons::NEXT, icon_color, 16.0),
            Message::Next,
            palette,
            false
        ),
        icon_button(
            icon(&icons::SHUFFLE, shuffle_color, 16.0),
            Message::ToggleShuffle,
            palette,
            shuffle
        ),
        icon_button(
            repeat_icon,
            Message::CycleRepeat,
            palette,
            repeat != RepeatMode::Off
        ),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);

    let total = track.map(|track| track.duration).unwrap_or_default();
    let total_secs = total.as_secs_f32();
    let range_end = if total_secs > 0.0 { total_secs } else { 1.0 };
    let seek_value = seek_drag
        .unwrap_or(position.as_secs_f32())
        .clamp(0.0, range_end);
    let shown = Duration::from_secs_f32(seek_value);

    let volume_percent = format!("{}%", (volume * 100.0).round() as u32);
    let volume_row = row![
        text("Vol").size(12).color(palette.muted),
        slider(0.0..=config::MAX_VOLUME, volume, Message::VolumeChanged)
            .on_release(Message::VolumeReleased)
            .step(0.01_f32)
            .width(160),
        text(volume_percent).size(12).color(palette.muted).width(40),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let seek_row = row![
        text(format_time(shown))
            .size(12)
            .color(palette.muted)
            .width(52),
        slider(0.0..=range_end, seek_value, Message::SeekChanged)
            .on_release(Message::SeekReleased)
            .step(0.1_f32)
            .width(Length::Fill),
        text(format_time(total))
            .size(12)
            .color(palette.muted)
            .width(52),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    container(
        column![
            row![
                container(now_playing::view(track, cover, loading, palette))
                    .width(Length::Fill)
                    .align_y(iced::Alignment::Center),
                transport,
                container(volume_row)
                    .width(Length::Fill)
                    .align_x(iced::Alignment::End)
                    .align_y(iced::Alignment::Center),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
            seek_row,
        ]
        .spacing(8),
    )
    .padding(12)
    .width(Length::Fill)
    .style(theme::surface_panel(palette))
    .into()
}

fn skip_button<'a>(
    handle: &iced::widget::svg::Handle,
    secs: u32,
    message: Message,
    palette: AppPalette,
) -> Element<'a, Message> {
    let content = row![
        icon(handle, palette.text, 16.0),
        text(secs.to_string()).size(12).color(palette.text),
    ]
    .spacing(2)
    .align_y(iced::Alignment::Center);
    icon_button(content.into(), message, palette, false)
}

fn icon_button<'a>(
    content: Element<'a, Message>,
    message: Message,
    palette: AppPalette,
    emphasized: bool,
) -> Element<'a, Message> {
    button(content)
        .on_press(message)
        .padding(8)
        .style(theme::icon_button(palette, emphasized))
        .into()
}
