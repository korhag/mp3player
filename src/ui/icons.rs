// ------------------------------------------------------------
// Module: icons
// Purpose: Embedded SVG handles for the transport controls.
// ------------------------------------------------------------

use std::sync::LazyLock;

use iced::widget::svg::{self, Handle};
use iced::{Color, Element};

use crate::app::Message;

fn handle(bytes: &'static [u8]) -> Handle {
    Handle::from_memory(bytes)
}

pub static PLAY: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/play.svg")));
pub static PAUSE: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/pause.svg")));
pub static PREVIOUS: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/previous.svg")));
pub static NEXT: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/next.svg")));
pub static SHUFFLE: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/shuffle.svg")));
pub static REPEAT: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/repeat.svg")));
pub static REPEAT_ONE: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/repeat_one.svg")));
pub static TRASH: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/trash.svg")));
pub static NOTE: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/note.svg")));
pub static STAR: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/star.svg")));
pub static STAR_FILLED: LazyLock<Handle> =
    LazyLock::new(|| handle(include_bytes!("../../assets/icons/star_filled.svg")));

// ------------------------------------------------------------
// Function: icon
// Purpose: Draws a monochrome SVG tinted to the theme color.
// ------------------------------------------------------------
pub fn icon<'a>(handle: &Handle, color: Color, size: f32) -> Element<'a, Message> {
    svg::Svg::new(handle.clone())
        .width(size)
        .height(size)
        .style(move |_theme, _status| svg::Style { color: Some(color) })
        .into()
}
