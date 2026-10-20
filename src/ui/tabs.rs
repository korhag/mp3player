// ------------------------------------------------------------
// Module: tabs
// Purpose: Library and Podcasts switch at the top of the window.
//          The Soundkeep logo sits on the left. Playback stays
//          visible on both tabs.
// ------------------------------------------------------------

use std::sync::{Mutex, OnceLock};

use iced::widget::{button, image, row, text};
use iced::{Color, Element, Length};

use crate::app::{Message, Tab};
use crate::theme::{self, AppPalette, ThemeId};

const LOGO_HEIGHT: f32 = 36.0;

struct LogoPixels {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

pub fn view(
    selected: Tab,
    theme_id: ThemeId,
    palette: AppPalette,
) -> Element<'static, Message> {
    row![
        logo(theme_id),
        row![
            tab("Library", Tab::Library, selected, palette),
            tab("Podcasts", Tab::Podcasts, selected, palette),
            tab("Settings", Tab::Settings, selected, palette),
        ]
        .spacing(8),
    ]
    .spacing(16)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill)
    .into()
}

// ------------------------------------------------------------
// Function: logo
// Purpose: Draws the mark and the Soundkeep name. The name is
//          recolored to the theme text so it stays readable.
// ------------------------------------------------------------
fn logo(theme_id: ThemeId) -> Element<'static, Message> {
    let source = logo_pixels();
    let aspect = source.width as f32 / source.height.max(1) as f32;
    image(logo_handle(theme_id))
        .width(LOGO_HEIGHT * aspect)
        .height(LOGO_HEIGHT)
        .content_fit(iced::ContentFit::Contain)
        .into()
}

// ------------------------------------------------------------
// Function: logo_handle
// Purpose: Returns the recolored logo for this theme. The large
//          image is rebuilt only when the theme changes.
// ------------------------------------------------------------
fn logo_handle(theme_id: ThemeId) -> image::Handle {
    static CACHE: Mutex<Option<(ThemeId, image::Handle)>> = Mutex::new(None);
    let mut cache = CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((cached_id, handle)) = cache.as_ref() {
        if *cached_id == theme_id {
            return handle.clone();
        }
    }
    let handle = recolored_logo(theme_id.palette().text);
    *cache = Some((theme_id, handle.clone()));
    handle
}

// ------------------------------------------------------------
// Function: recolored_logo
// Purpose: Copies the logo and paints near-black wordmark pixels
//          with the theme text color. The jade mark is unchanged.
// ------------------------------------------------------------
fn recolored_logo(text_color: Color) -> image::Handle {
    let source = logo_pixels();
    let mut pixels = source.pixels.clone();
    let red = color_channel(text_color.r);
    let green = color_channel(text_color.g);
    let blue = color_channel(text_color.b);
    for chunk in pixels.chunks_exact_mut(4) {
        if is_wordmark(chunk[0], chunk[1], chunk[2], chunk[3]) {
            chunk[0] = red;
            chunk[1] = green;
            chunk[2] = blue;
        }
    }
    image::Handle::from_rgba(source.width, source.height, pixels)
}

// ------------------------------------------------------------
// Function: logo_pixels
// Purpose: Decodes the branding logo once and drops the
//          transparent margin so the header height is the artwork.
// ------------------------------------------------------------
fn logo_pixels() -> &'static LogoPixels {
    static PIXELS: OnceLock<LogoPixels> = OnceLock::new();
    PIXELS.get_or_init(|| {
        let image = ::image::load_from_memory(include_bytes!("../../assets/branding/logo.png"))
            .expect("decode assets/branding/logo.png")
            .to_rgba8();
        let (width, height) = image.dimensions();
        let raw = image.into_raw();
        let (min_x, min_y, crop_width, crop_height) = opaque_bounds(&raw, width, height);
        let mut pixels = Vec::with_capacity((crop_width * crop_height * 4) as usize);
        for y in min_y..min_y + crop_height {
            let row = (y * width + min_x) as usize * 4;
            let end = row + crop_width as usize * 4;
            pixels.extend_from_slice(&raw[row..end]);
        }
        LogoPixels {
            width: crop_width,
            height: crop_height,
            pixels,
        }
    })
}

// ------------------------------------------------------------
// Function: opaque_bounds
// Purpose: Finds the rectangle that contains every visible pixel.
// ------------------------------------------------------------
fn opaque_bounds(pixels: &[u8], width: u32, height: u32) -> (u32, u32, u32, u32) {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    for y in 0..height {
        for x in 0..width {
            let alpha = pixels[((y * width + x) * 4 + 3) as usize];
            if alpha == 0 {
                continue;
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if min_x > max_x {
        return (0, 0, width, height);
    }
    (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
}

// ------------------------------------------------------------
// Function: is_wordmark
// Purpose: True for the near-black Soundkeep letters. Jade mark
//          pixels are greener and stay as drawn.
// ------------------------------------------------------------
fn is_wordmark(red: u8, green: u8, blue: u8, alpha: u8) -> bool {
    if alpha == 0 {
        return false;
    }
    let teal = green > 80 && green > red.saturating_add(40);
    !teal && red < 96 && green < 96 && blue < 96
}

fn color_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
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

#[cfg(test)]
mod tests {
    use super::{is_wordmark, logo_pixels};

    #[test]
    fn wordmark_pixels_are_recolored_and_the_jade_mark_is_kept() {
        assert!(is_wordmark(16, 32, 32, 255));
        assert!(!is_wordmark(0, 144, 128, 255));
        assert!(!is_wordmark(0, 0, 0, 0));
    }

    #[test]
    fn logo_file_decodes_and_crops_to_the_artwork() {
        let logo = logo_pixels();
        assert!(logo.width > logo.height);
        assert_eq!(logo.pixels.len(), (logo.width * logo.height * 4) as usize);
        assert!(logo.pixels.chunks(4).any(|pixel| pixel[3] > 0));
    }
}
