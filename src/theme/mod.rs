// ------------------------------------------------------------
// Module: theme
// Purpose: Turns a palette into an iced Theme and widget styles.
// ------------------------------------------------------------

mod palettes;

pub use palettes::{AppPalette, ThemeId};

use iced::border::Border;
use iced::widget::{button, container};
use iced::{Background, Color, Theme};

// ------------------------------------------------------------
// Function: iced_theme
// Purpose: Builds the iced theme used by sliders, scrollbars,
//          and text defaults.
// ------------------------------------------------------------
pub fn iced_theme(theme_id: ThemeId) -> Theme {
    let palette = theme_id.palette();
    // Sliders, scrollbars, and text fields follow this base palette.
    // Custom panels use `AppPalette` directly so surfaces stay exact.
    Theme::custom(
        theme_id.label(),
        iced::theme::Palette {
            background: palette.background,
            text: palette.text,
            primary: palette.accent,
            success: palette.accent,
            danger: palette.danger,
            warning: palette.muted,
        },
    )
}

// ------------------------------------------------------------
// Function: app_background
// Purpose: Full-window background fill.
// ------------------------------------------------------------
pub fn app_background(palette: AppPalette) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(palette.background)),
        text_color: Some(palette.text),
        ..Default::default()
    }
}

// ------------------------------------------------------------
// Function: surface_panel
// Purpose: Sidebar and queue panel.
// ------------------------------------------------------------
pub fn surface_panel(palette: AppPalette) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(palette.surface)),
        text_color: Some(palette.text),
        border: Border {
            color: palette.surface_alt,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

// ------------------------------------------------------------
// Function: track_row
// Purpose: One queue row. The current row uses the alt surface.
// ------------------------------------------------------------
pub fn track_row(palette: AppPalette, current: bool) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(if current {
            palette.surface_alt
        } else {
            palette.surface
        })),
        text_color: Some(palette.text),
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// ------------------------------------------------------------
// Function: episode_row
// Purpose: An episode line. The episode that is loaded in the
//          player gets a filled row and an accent outline so it
//          can be told apart from the rest of the list.
// ------------------------------------------------------------
pub fn episode_row(
    palette: AppPalette,
    current: bool,
    selected: bool,
) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: (current || selected).then_some(Background::Color(palette.surface_alt)),
        text_color: Some(palette.text),
        border: Border {
            color: if current {
                palette.accent
            } else {
                Color::TRANSPARENT
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

// ------------------------------------------------------------
// Function: drop_slot
// Purpose: The insertion line shown while a favorite is dragged.
// ------------------------------------------------------------
pub fn drop_slot(palette: AppPalette, active: bool) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(if active {
            palette.accent
        } else {
            palette.surface_alt
        })),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// ------------------------------------------------------------
// Function: group_rail
// Purpose: The vertical line beside the shows in an open group.
// ------------------------------------------------------------
pub fn group_rail(palette: AppPalette) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(palette.muted)),
        border: Border {
            radius: 1.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn button_style(background: Option<Color>, text: Color) -> button::Style {
    button::Style {
        background: background.map(Background::Color),
        text_color: text,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// ------------------------------------------------------------
// Function: primary_button
// Purpose: Filled accent button for Save and Add.
// ------------------------------------------------------------
pub fn primary_button(palette: AppPalette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => palette.accent,
            button::Status::Disabled => palette.surface_alt,
            _ => palette.accent,
        };
        let text = match status {
            button::Status::Disabled => palette.muted,
            _ => palette.on_accent,
        };
        button_style(Some(background), text)
    }
}

// ------------------------------------------------------------
// Function: quiet_button
// Purpose: Low-emphasis text button.
// ------------------------------------------------------------
pub fn quiet_button(palette: AppPalette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => Some(palette.surface_alt),
            _ => None,
        };
        button_style(background, palette.text)
    }
}

// ------------------------------------------------------------
// Function: danger_button
// Purpose: Delete, colored only on hover.
// ------------------------------------------------------------
pub fn danger_button(palette: AppPalette) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => Some(palette.surface_alt),
            _ => None,
        };
        button_style(background, palette.danger)
    }
}

// ------------------------------------------------------------
// Function: icon_button
// Purpose: Transport button. `emphasized` draws the accent color.
// ------------------------------------------------------------
pub fn icon_button(
    palette: AppPalette,
    emphasized: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => Some(palette.surface_alt),
            _ => None,
        };
        let text = if emphasized {
            palette.accent
        } else {
            palette.text
        };
        button_style(background, text)
    }
}
