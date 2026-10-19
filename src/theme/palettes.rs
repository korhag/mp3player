// ------------------------------------------------------------
// Module: palettes
// Purpose: Static color sets for the player, copied from the
//          Omarchy themes installed on this machine. The app
//          does not read those theme folders at runtime, so the
//          same themes work on a machine without Omarchy.
//
//          A palette maps Omarchy roles onto AppPalette:
//          background, lighter background (surface), selection
//          (surface_alt), foreground (text), accent, and red
//          (danger). Muted text is the more readable of the
//          theme's muted and dark-foreground colors. When both
//          of those are too dim to read, the palette uses
//          another text color from the same theme. on_accent is
//          whichever of the background and the foreground
//          contrasts more with the accent.
//
//          A few themes have no separate panel color, or a
//          selection color that matches the text. Those use a
//          nearby shade from the same file so rows stay readable.
//          Add a variant and a palette() arm to ship another theme.
//          Keep existing variant names: settings store them, and
//          an unknown name makes the whole settings file fail.
// ------------------------------------------------------------

use iced::Color;
use serde::{Deserialize, Serialize};

// ------------------------------------------------------------
// Enum: ThemeId
// Purpose: The theme stored in settings and shown in the picker.
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeId {
    Blackgold,
    Blackturq,
    CastleOnALake,
    CatppuccinMocha,
    CatppuccinLatte,
    Darcula,
    Ethereal,
    Everforest,
    FlexokiLight,
    GoldRush,
    GoldenBrown,
    GruvboxDark,
    Hackerman,
    Kanagawa,
    LastHorizon,
    Lumon,
    Lupine,
    #[default]
    MatteBlack,
    Miasma,
    Neovoid,
    Nord,
    Omacarchy,
    Omaled,
    OsakaJade,
    Retro82,
    Ristretto,
    RosePineDawn,
    Solitude,
    TokyoNight,
    Vantablack,
    White,
}

impl ThemeId {
    pub const ALL: [ThemeId; 31] = [
        ThemeId::Blackgold,
        ThemeId::Blackturq,
        ThemeId::CastleOnALake,
        ThemeId::CatppuccinMocha,
        ThemeId::CatppuccinLatte,
        ThemeId::Darcula,
        ThemeId::Ethereal,
        ThemeId::Everforest,
        ThemeId::FlexokiLight,
        ThemeId::GoldRush,
        ThemeId::GoldenBrown,
        ThemeId::GruvboxDark,
        ThemeId::Hackerman,
        ThemeId::Kanagawa,
        ThemeId::LastHorizon,
        ThemeId::Lumon,
        ThemeId::Lupine,
        ThemeId::MatteBlack,
        ThemeId::Miasma,
        ThemeId::Neovoid,
        ThemeId::Nord,
        ThemeId::Omacarchy,
        ThemeId::Omaled,
        ThemeId::OsakaJade,
        ThemeId::Retro82,
        ThemeId::Ristretto,
        ThemeId::RosePineDawn,
        ThemeId::Solitude,
        ThemeId::TokyoNight,
        ThemeId::Vantablack,
        ThemeId::White,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeId::Blackgold => "Blackgold",
            ThemeId::Blackturq => "Blackturq",
            ThemeId::CastleOnALake => "Castle On A Lake",
            ThemeId::CatppuccinMocha => "Catppuccin",
            ThemeId::CatppuccinLatte => "Catppuccin Latte",
            ThemeId::Darcula => "Darcula",
            ThemeId::Ethereal => "Ethereal",
            ThemeId::Everforest => "Everforest",
            ThemeId::FlexokiLight => "Flexoki Light",
            ThemeId::GoldRush => "Gold Rush",
            ThemeId::GoldenBrown => "Golden Brown",
            ThemeId::GruvboxDark => "Gruvbox",
            ThemeId::Hackerman => "Hackerman",
            ThemeId::Kanagawa => "Kanagawa",
            ThemeId::LastHorizon => "Last Horizon",
            ThemeId::Lumon => "Lumon",
            ThemeId::Lupine => "Lupine",
            ThemeId::MatteBlack => "Matte Black",
            ThemeId::Miasma => "Miasma",
            ThemeId::Neovoid => "Neovoid",
            ThemeId::Nord => "Nord",
            ThemeId::Omacarchy => "Omacarchy",
            ThemeId::Omaled => "Omaled",
            ThemeId::OsakaJade => "Osaka Jade",
            ThemeId::Retro82 => "Retro 82",
            ThemeId::Ristretto => "Ristretto",
            ThemeId::RosePineDawn => "Rose Pine",
            ThemeId::Solitude => "Solitude",
            ThemeId::TokyoNight => "Tokyo Night",
            ThemeId::Vantablack => "Vantablack",
            ThemeId::White => "White",
        }
    }

    // --------------------------------------------------------
    // Function: palette
    // Purpose: Concrete colors for this theme.
    // --------------------------------------------------------
    pub fn palette(self) -> AppPalette {
        match self {
            ThemeId::Blackgold => AppPalette {
                background: rgb(0x0d0d0d),
                surface: rgb(0x1f1a12),
                surface_alt: rgb(0x241f16),
                text: rgb(0xebdbb2),
                muted: rgb(0xdcd3b1),
                accent: rgb(0x6e6a58),
                on_accent: rgb(0xebdbb2),
                danger: rgb(0xd35f5f),
                is_dark: true,
            },
            ThemeId::Blackturq => AppPalette {
                background: rgb(0x0a0a0a),
                surface: rgb(0x14201e),
                surface_alt: rgb(0x182826),
                text: rgb(0xc8dcdc),
                muted: rgb(0xaabab6),
                accent: rgb(0xadf0e9),
                on_accent: rgb(0x0a0a0a),
                danger: rgb(0xd35f5f),
                is_dark: true,
            },
            // The theme's selection color is the same pink as the
            // foreground, so the alt surface uses its muted gray.
            ThemeId::CastleOnALake => AppPalette {
                background: rgb(0x060606),
                surface: rgb(0x1f1f1e),
                surface_alt: rgb(0x626262),
                text: rgb(0xf0d4e4),
                muted: rgb(0xb49faa),
                accent: rgb(0x9b6b9f),
                on_accent: rgb(0x060606),
                danger: rgb(0xa06048),
                is_dark: true,
            },
            ThemeId::CatppuccinMocha => AppPalette {
                background: rgb(0x1e1e2e),
                surface: rgb(0x313244),
                surface_alt: rgb(0x45475a),
                text: rgb(0xcdd6f4),
                muted: rgb(0x6c7086),
                accent: rgb(0x89b4fa),
                on_accent: rgb(0x1e1e2e),
                danger: rgb(0xf38ba8),
                is_dark: true,
            },
            ThemeId::CatppuccinLatte => AppPalette {
                background: rgb(0xeff1f5),
                surface: rgb(0xdce0e8),
                surface_alt: rgb(0xccd0da),
                text: rgb(0x4c4f69),
                muted: rgb(0x5c5f77),
                accent: rgb(0x1e66f5),
                on_accent: rgb(0xeff1f5),
                danger: rgb(0xd20f39),
                is_dark: false,
            },
            ThemeId::Darcula => AppPalette {
                background: rgb(0x2b2b2b),
                surface: rgb(0x3c3f41),
                surface_alt: rgb(0x3f4b6e),
                text: rgb(0xa9b7c6),
                muted: rgb(0x808080),
                accent: rgb(0xcc7832),
                on_accent: rgb(0x2b2b2b),
                danger: rgb(0xff6b68),
                is_dark: true,
            },
            ThemeId::Ethereal => AppPalette {
                background: rgb(0x060b1e),
                surface: rgb(0x131a3a),
                surface_alt: rgb(0x252e56),
                text: rgb(0xffcead),
                muted: rgb(0x6d7db6),
                accent: rgb(0x7d82d9),
                on_accent: rgb(0x060b1e),
                danger: rgb(0xed5b5a),
                is_dark: true,
            },
            ThemeId::Everforest => AppPalette {
                background: rgb(0x2d353b),
                surface: rgb(0x343f44),
                surface_alt: rgb(0x3d484d),
                text: rgb(0xd3c6aa),
                muted: rgb(0x9da9a0),
                accent: rgb(0x7fbbb3),
                on_accent: rgb(0x2d353b),
                danger: rgb(0xe67e80),
                is_dark: true,
            },
            ThemeId::FlexokiLight => AppPalette {
                background: rgb(0xfffcf0),
                surface: rgb(0xe6e4d9),
                surface_alt: rgb(0xcecdc3),
                text: rgb(0x100f0f),
                muted: rgb(0x878580),
                accent: rgb(0x205ea6),
                on_accent: rgb(0xfffcf0),
                danger: rgb(0xd14d41),
                is_dark: false,
            },
            ThemeId::GoldRush => AppPalette {
                background: rgb(0x121212),
                surface: rgb(0x76520e),
                surface_alt: rgb(0x926c15),
                text: rgb(0xd9d9d9),
                muted: rgb(0xa47e1b),
                accent: rgb(0xc9a227),
                on_accent: rgb(0x121212),
                danger: rgb(0xdbb42c),
                is_dark: true,
            },
            ThemeId::GoldenBrown => AppPalette {
                background: rgb(0x0f0b05),
                surface: rgb(0x231b0e),
                surface_alt: rgb(0x57462c),
                text: rgb(0xc3a983),
                muted: rgb(0xa88c62),
                accent: rgb(0xc3a983),
                on_accent: rgb(0x0f0b05),
                danger: rgb(0xa88c62),
                is_dark: true,
            },
            ThemeId::GruvboxDark => AppPalette {
                background: rgb(0x282828),
                surface: rgb(0x3c3836),
                surface_alt: rgb(0x504945),
                text: rgb(0xd4be98),
                muted: rgb(0x7c6f64),
                accent: rgb(0x7daea3),
                on_accent: rgb(0x282828),
                danger: rgb(0xea6962),
                is_dark: true,
            },
            ThemeId::Hackerman => AppPalette {
                background: rgb(0x0b0c16),
                surface: rgb(0x151828),
                surface_alt: rgb(0x1f253a),
                text: rgb(0xddf7ff),
                muted: rgb(0x6a6e95),
                accent: rgb(0x82fb9c),
                on_accent: rgb(0x0b0c16),
                danger: rgb(0x50f872),
                is_dark: true,
            },
            ThemeId::Kanagawa => AppPalette {
                background: rgb(0x1f1f28),
                surface: rgb(0x223249),
                surface_alt: rgb(0x363646),
                text: rgb(0xdcd7ba),
                muted: rgb(0x727169),
                accent: rgb(0xdcd7ba),
                on_accent: rgb(0x1f1f28),
                danger: rgb(0xc34043),
                is_dark: true,
            },
            // lighter_background is the same as the background.
            ThemeId::LastHorizon => AppPalette {
                background: rgb(0x0c0b0c),
                surface: rgb(0x1a1819),
                surface_alt: rgb(0x584e51),
                text: rgb(0xfafcfb),
                muted: rgb(0xcfd3cd),
                accent: rgb(0xb59790),
                on_accent: rgb(0x0c0b0c),
                danger: rgb(0xc38b7b),
                is_dark: true,
            },
            ThemeId::Lumon => AppPalette {
                background: rgb(0x16242d),
                surface: rgb(0x1b2d40),
                surface_alt: rgb(0x243d56),
                text: rgb(0xd6e2ee),
                muted: rgb(0x4d86b0),
                accent: rgb(0x8bc9eb),
                on_accent: rgb(0x16242d),
                danger: rgb(0x4d86b0),
                is_dark: true,
            },
            ThemeId::Lupine => AppPalette {
                background: rgb(0xfafafa),
                surface: rgb(0xf5f5f5),
                surface_alt: rgb(0xd0d0d0),
                text: rgb(0x212121),
                muted: rgb(0x757575),
                accent: rgb(0x3264eb),
                on_accent: rgb(0xfafafa),
                danger: rgb(0xc900c4),
                is_dark: false,
            },
            ThemeId::MatteBlack => AppPalette {
                background: rgb(0x121212),
                surface: rgb(0x1e1e1e),
                surface_alt: rgb(0x2a2a2a),
                text: rgb(0xbebebe),
                muted: rgb(0x8a8a8d),
                accent: rgb(0xe68e0d),
                on_accent: rgb(0x121212),
                danger: rgb(0xd35f5f),
                is_dark: true,
            },
            ThemeId::Miasma => AppPalette {
                background: rgb(0x222222),
                surface: rgb(0x2c2c2c),
                surface_alt: rgb(0x383838),
                text: rgb(0xc2c2b0),
                muted: rgb(0x8a8a7e),
                accent: rgb(0x78824b),
                on_accent: rgb(0x222222),
                danger: rgb(0x685742),
                is_dark: true,
            },
            ThemeId::Neovoid => AppPalette {
                background: rgb(0x000000),
                surface: rgb(0x1a1a1a),
                surface_alt: rgb(0x333333),
                text: rgb(0xe0e0e0),
                muted: rgb(0x8a8a8d),
                accent: rgb(0x00bfff),
                on_accent: rgb(0x000000),
                danger: rgb(0xff0040),
                is_dark: true,
            },
            ThemeId::Nord => AppPalette {
                background: rgb(0x2e3440),
                surface: rgb(0x3b4252),
                surface_alt: rgb(0x434c5e),
                text: rgb(0xd8dee9),
                muted: rgb(0xadb5c4),
                accent: rgb(0x81a1c1),
                on_accent: rgb(0x2e3440),
                danger: rgb(0xbf616a),
                is_dark: true,
            },
            // Grayscale theme: its ANSI red sits too close to the
            // background to read, so danger uses the accent.
            ThemeId::Omacarchy => AppPalette {
                background: rgb(0x1c1c1e),
                surface: rgb(0x2c2c2e),
                surface_alt: rgb(0x3a3a3c),
                text: rgb(0xffffff),
                muted: rgb(0xa0a0a0),
                accent: rgb(0x8e8e93),
                on_accent: rgb(0x1c1c1e),
                danger: rgb(0x8e8e93),
                is_dark: true,
            },
            ThemeId::Omaled => AppPalette {
                background: rgb(0x000000),
                surface: rgb(0x5a000b),
                surface_alt: rgb(0x860111),
                text: rgb(0xe66063),
                muted: rgb(0xdd2c2f),
                accent: rgb(0x860111),
                on_accent: rgb(0xe66063),
                danger: rgb(0xdd2c2f),
                is_dark: true,
            },
            ThemeId::OsakaJade => AppPalette {
                background: rgb(0x111c18),
                surface: rgb(0x23372b),
                surface_alt: rgb(0x32473b),
                text: rgb(0xc1c497),
                muted: rgb(0x81b8a8),
                accent: rgb(0x509475),
                on_accent: rgb(0x111c18),
                danger: rgb(0xff5345),
                is_dark: true,
            },
            ThemeId::Retro82 => AppPalette {
                background: rgb(0x05182e),
                surface: rgb(0x0a2540),
                surface_alt: rgb(0x134e5a),
                text: rgb(0xf6dcac),
                muted: rgb(0x3f8f8a),
                accent: rgb(0xfaa968),
                on_accent: rgb(0x05182e),
                danger: rgb(0xf85525),
                is_dark: true,
            },
            ThemeId::Ristretto => AppPalette {
                background: rgb(0x2c2525),
                surface: rgb(0x3d2f2a),
                surface_alt: rgb(0x403e41),
                text: rgb(0xe6d9db),
                muted: rgb(0xc3b7b8),
                accent: rgb(0xf38d70),
                on_accent: rgb(0x2c2525),
                danger: rgb(0xfd6883),
                is_dark: true,
            },
            ThemeId::RosePineDawn => AppPalette {
                background: rgb(0xfaf4ed),
                surface: rgb(0xf2e9e1),
                surface_alt: rgb(0xdfdad9),
                text: rgb(0x575279),
                muted: rgb(0x6e6a86),
                accent: rgb(0x56949f),
                on_accent: rgb(0xfaf4ed),
                danger: rgb(0xb4637a),
                is_dark: false,
            },
            // lighter_background is the same as the background.
            ThemeId::Solitude => AppPalette {
                background: rgb(0x101315),
                surface: rgb(0x1c2225),
                surface_alt: rgb(0x343d41),
                text: rgb(0xcacccc),
                muted: rgb(0xa5aeb4),
                accent: rgb(0x798186),
                on_accent: rgb(0x101315),
                danger: rgb(0x565d60),
                is_dark: true,
            },
            ThemeId::TokyoNight => AppPalette {
                background: rgb(0x1a1b26),
                surface: rgb(0x24283b),
                surface_alt: rgb(0x292e42),
                text: rgb(0xa9b1d6),
                muted: rgb(0x565f89),
                accent: rgb(0x7aa2f7),
                on_accent: rgb(0x1a1b26),
                danger: rgb(0xf7768e),
                is_dark: true,
            },
            // lighter_background is the same as the selection.
            ThemeId::Vantablack => AppPalette {
                background: rgb(0x000000),
                surface: rgb(0x090909),
                surface_alt: rgb(0x1a1a1a),
                text: rgb(0xffffff),
                muted: rgb(0x7a7a7a),
                accent: rgb(0x8d8d8d),
                on_accent: rgb(0x000000),
                danger: rgb(0xa4a4a4),
                is_dark: true,
            },
            // lighter_background is the selection gray, which is
            // too strong for a panel. The surface uses the theme's
            // dark background instead.
            ThemeId::White => AppPalette {
                background: rgb(0xffffff),
                surface: rgb(0xf5f5f5),
                surface_alt: rgb(0xc0c0c0),
                text: rgb(0x000000),
                muted: rgb(0x808080),
                accent: rgb(0x6e6e6e),
                on_accent: rgb(0xffffff),
                danger: rgb(0x2a2a2a),
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

    // --------------------------------------------------------
    // Function: channel
    // Purpose: WCAG sRGB channel to linear light.
    // --------------------------------------------------------
    fn channel(value: f32) -> f32 {
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    // --------------------------------------------------------
    // Function: contrast
    // Purpose: WCAG contrast ratio between two colors.
    // --------------------------------------------------------
    fn contrast(first: Color, second: Color) -> f32 {
        let luminance = |color: Color| {
            0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
        };
        let lighter = luminance(first).max(luminance(second));
        let darker = luminance(first).min(luminance(second));
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn every_theme_has_a_distinct_label() {
        let mut labels: Vec<_> = ThemeId::ALL.iter().map(|theme| theme.label()).collect();
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), ThemeId::ALL.len());
        assert!(!ThemeId::RosePineDawn.palette().is_dark);
        assert!(ThemeId::OsakaJade.palette().is_dark);
    }

    #[test]
    fn light_themes_match_the_omarchy_light_mode() {
        let light: Vec<_> = ThemeId::ALL
            .iter()
            .filter(|theme| !theme.palette().is_dark)
            .map(|theme| theme.label())
            .collect();
        assert_eq!(
            light,
            vec![
                "Catppuccin Latte",
                "Flexoki Light",
                "Lupine",
                "Rose Pine",
                "White",
            ]
        );
    }

    #[test]
    fn text_and_accent_stay_readable() {
        for theme in ThemeId::ALL {
            let palette = theme.palette();
            let text_contrast = contrast(palette.text, palette.background);
            let accent_contrast = contrast(palette.on_accent, palette.accent);
            assert!(
                text_contrast >= 4.5,
                "{} text contrast {text_contrast:.2} is below 4.5",
                theme.label()
            );
            assert!(
                accent_contrast >= 3.0,
                "{} accent contrast {accent_contrast:.2} is below 3",
                theme.label()
            );
        }
    }

    #[test]
    fn legacy_theme_name_still_loads() {
        for theme in ThemeId::ALL {
            let json = serde_json::to_string(&theme).unwrap();
            let loaded: ThemeId = serde_json::from_str(&json).unwrap();
            assert_eq!(loaded, theme);
        }

        let settings: crate::settings::Settings =
            serde_json::from_str(r#"{ "theme": "RosePineDawn" }"#).unwrap();
        assert_eq!(settings.theme, ThemeId::RosePineDawn);
        assert_eq!(settings.theme.label(), "Rose Pine");
    }
}
