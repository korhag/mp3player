// ------------------------------------------------------------
// Module: palettes
// Purpose: Named color sets for the player. Add a variant here
//          and a palette in `palette()` to ship another theme.
// ------------------------------------------------------------

use iced::Color;
use serde::{Deserialize, Serialize};

// ------------------------------------------------------------
// Enum: ThemeId
// Purpose: The theme stored in settings and shown in the picker.
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeId {
    #[default]
    MatteBlack,
    OsakaJade,
    TokyoNight,
    CatppuccinMocha,
    GruvboxDark,
    Nord,
    RosePineDawn,
}

impl ThemeId {
    pub const ALL: [ThemeId; 7] = [
        ThemeId::MatteBlack,
        ThemeId::OsakaJade,
        ThemeId::TokyoNight,
        ThemeId::CatppuccinMocha,
        ThemeId::GruvboxDark,
        ThemeId::Nord,
        ThemeId::RosePineDawn,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeId::MatteBlack => "Matte Black",
            ThemeId::OsakaJade => "Osaka Jade",
            ThemeId::TokyoNight => "Tokyo Night",
            ThemeId::CatppuccinMocha => "Catppuccin Mocha",
            ThemeId::GruvboxDark => "Gruvbox Dark",
            ThemeId::Nord => "Nord",
            ThemeId::RosePineDawn => "Rose Pine Dawn",
        }
    }

    // --------------------------------------------------------
    // Function: palette
    // Purpose: Concrete colors for this theme.
    // --------------------------------------------------------
    pub fn palette(self) -> AppPalette {
        match self {
            ThemeId::MatteBlack => AppPalette {
                background: rgb(0x121212),
                surface: rgb(0x1b1b1b),
                surface_alt: rgb(0x262626),
                text: rgb(0xf2f2f2),
                muted: rgb(0x8d8d8d),
                accent: rgb(0xe6e6e6),
                on_accent: rgb(0x121212),
                danger: rgb(0xd46a6a),
                is_dark: true,
            },
            ThemeId::OsakaJade => AppPalette {
                background: rgb(0x0d1714),
                surface: rgb(0x14211d),
                surface_alt: rgb(0x1c2e28),
                text: rgb(0xe8f4ef),
                muted: rgb(0x7f9c92),
                accent: rgb(0x3ecf9a),
                on_accent: rgb(0x06241a),
                danger: rgb(0xe07a7a),
                is_dark: true,
            },
            ThemeId::TokyoNight => AppPalette {
                background: rgb(0x1a1b26),
                surface: rgb(0x24283b),
                surface_alt: rgb(0x2f3549),
                text: rgb(0xc0caf5),
                muted: rgb(0x565f89),
                accent: rgb(0x7aa2f7),
                on_accent: rgb(0x1a1b26),
                danger: rgb(0xf7768e),
                is_dark: true,
            },
            ThemeId::CatppuccinMocha => AppPalette {
                background: rgb(0x1e1e2e),
                surface: rgb(0x313244),
                surface_alt: rgb(0x45475a),
                text: rgb(0xcdd6f4),
                muted: rgb(0x6c7086),
                accent: rgb(0xcba6f7),
                on_accent: rgb(0x1e1e2e),
                danger: rgb(0xf38ba8),
                is_dark: true,
            },
            ThemeId::GruvboxDark => AppPalette {
                background: rgb(0x282828),
                surface: rgb(0x3c3836),
                surface_alt: rgb(0x504945),
                text: rgb(0xebdbb2),
                muted: rgb(0xa89984),
                accent: rgb(0xfe8019),
                on_accent: rgb(0x282828),
                danger: rgb(0xfb4934),
                is_dark: true,
            },
            ThemeId::Nord => AppPalette {
                background: rgb(0x2e3440),
                surface: rgb(0x3b4252),
                surface_alt: rgb(0x434c5e),
                text: rgb(0xeceff4),
                muted: rgb(0x9aa4b8),
                accent: rgb(0x88c0d0),
                on_accent: rgb(0x2e3440),
                danger: rgb(0xbf616a),
                is_dark: true,
            },
            ThemeId::RosePineDawn => AppPalette {
                background: rgb(0xfaf4ed),
                surface: rgb(0xfffaf3),
                surface_alt: rgb(0xf2e9e1),
                text: rgb(0x575279),
                muted: rgb(0x797593),
                accent: rgb(0xd7827e),
                on_accent: rgb(0xfffaf3),
                danger: rgb(0xb4637a),
                is_dark: false,
            },
        }
    }
}

impl std::fmt::Display for ThemeId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

// ------------------------------------------------------------
// Struct: AppPalette
// Purpose: The colors a theme contributes to widgets.
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub struct AppPalette {
    pub background: Color,
    pub surface: Color,
    pub surface_alt: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub danger: Color,
    /// True for dark themes. Light themes set this to false.
    #[allow(dead_code)]
    pub is_dark: bool,
}

fn rgb(hex: u32) -> Color {
    Color::from_rgb(
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_has_a_distinct_label() {
        let mut labels: Vec<_> = ThemeId::ALL.iter().map(|theme| theme.label()).collect();
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), ThemeId::ALL.len());
        assert!(!ThemeId::RosePineDawn.palette().is_dark);
        assert!(ThemeId::OsakaJade.palette().is_dark);
    }
}
