// ------------------------------------------------------------
// Module: system
// Purpose: Opens a file in the system file manager, and opens
//          a URL in the default browser.
// ------------------------------------------------------------

use std::io;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

// ------------------------------------------------------------
// Function: reveal
// Purpose: Shows a file in Explorer, or its folder on Linux.
//          A missing file falls back to the parent folder.
// ------------------------------------------------------------
pub fn reveal(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if path.exists() {
            Command::new("explorer.exe")
                .raw_arg(format!("/select,\"{}\"", path.display()))
                .spawn()?;
        } else {
            let folder = path.parent().unwrap_or(path);
            Command::new("explorer.exe").arg(folder).spawn()?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let folder = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        Command::new("xdg-open").arg(folder).spawn()?;
        Ok(())
    }
}

// ------------------------------------------------------------
// Function: open_url
// Purpose: Opens a web page in the default browser.
// ------------------------------------------------------------
pub fn open_url(url: &str) -> io::Result<()> {
    #[cfg(windows)]
    {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
        return Ok(());
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Command::new("xdg-open").arg(url).spawn()?;
        Ok(())
    }
}

// ------------------------------------------------------------
// Function: icon_pixels
// Purpose: Crops the app mark to its visible pixels and scales
//          it to a square. Windows only accepts small icons.
// ------------------------------------------------------------
pub fn icon_pixels(size: u32) -> Option<Vec<u8>> {
    let source = ::image::load_from_memory(include_bytes!("../assets/icon.png"))
        .ok()?
        .to_rgba8();
    let (x, y, crop_width, crop_height) =
        opaque_bounds(&source).unwrap_or((0, 0, source.width(), source.height()));
    let cropped = ::image::imageops::crop_imm(&source, x, y, crop_width, crop_height).to_image();
    let side = crop_width.max(crop_height);
    let mut square = ::image::RgbaImage::from_pixel(side, side, ::image::Rgba([0, 0, 0, 0]));
    let offset_x = ((side - crop_width) / 2) as i64;
    let offset_y = ((side - crop_height) / 2) as i64;
    ::image::imageops::overlay(&mut square, &cropped, offset_x, offset_y);
    let resized =
        ::image::imageops::resize(&square, size, size, ::image::imageops::FilterType::Triangle);
    Some(resized.into_raw())
}

// ------------------------------------------------------------
// Function: opaque_bounds
// Purpose: Rectangle that contains every non-transparent pixel.
// ------------------------------------------------------------
fn opaque_bounds(image: &::image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = image.dimensions();
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        found = true;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    if !found {
        return None;
    }
    Some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
}

// ------------------------------------------------------------
// Function: apply_shell_icons
// Purpose: Puts the embedded icon on the Windows title bar and
//          taskbar. The window class itself has no icon, and
//          iced only tries the small one. Runs once.
// ------------------------------------------------------------
pub fn apply_shell_icons() {
    static APPLIED: OnceLock<()> = OnceLock::new();
    APPLIED.get_or_init(|| {
        #[cfg(windows)]
        apply_windows_shell_icons();
    });
}

#[cfg(windows)]
fn apply_windows_shell_icons() {
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    let small = load_or_create_icon(module, 32);
    let big = load_or_create_icon(module, 256);
    if small.is_null() && big.is_null() {
        return;
    }
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(enum_window, &mut windows as *mut Vec<Hwnd> as isize);
    }
    for hwnd in windows {
        unsafe {
            if !small.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL, small as isize);
            }
            if !big.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_BIG, big as isize);
            }
        }
    }
}

#[cfg(windows)]
fn load_or_create_icon(module: Hinstance, size: i32) -> Hicon {
    let loaded = unsafe { LoadImageW(module, 1 as *const u16, IMAGE_ICON, size, size, 0) };
    if !loaded.is_null() {
        return loaded;
    }
    let Some(pixels) = icon_pixels(size as u32) else {
        return std::ptr::null_mut();
    };
    create_icon(size, &pixels)
}

#[cfg(windows)]
fn create_icon(size: i32, pixels: &[u8]) -> Hicon {
    let mut xor_bits = pixels.to_vec();
    let mut and_mask = Vec::with_capacity(xor_bits.len() / 4);
    for chunk in xor_bits.chunks_exact_mut(4) {
        and_mask.push(chunk[3].wrapping_sub(u8::MAX));
        chunk.swap(0, 2);
    }
    unsafe {
        CreateIcon(
            std::ptr::null_mut(),
            size,
            size,
            1,
            32,
            and_mask.as_ptr(),
            xor_bits.as_ptr(),
        )
    }
}

#[cfg(windows)]
unsafe extern "system" fn enum_window(hwnd: Hwnd, param: isize) -> i32 {
    let found = unsafe { &mut *(param as *mut Vec<Hwnd>) };
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut process_id);
    }
    if process_id == unsafe { GetCurrentProcessId() } {
        found.push(hwnd);
    }
    1
}

#[cfg(windows)]
type Hinstance = *mut core::ffi::c_void;
#[cfg(windows)]
type Hicon = *mut core::ffi::c_void;
#[cfg(windows)]
type Hwnd = *mut core::ffi::c_void;

#[cfg(windows)]
const IMAGE_ICON: u32 = 1;
#[cfg(windows)]
const WM_SETICON: u32 = 0x0080;
#[cfg(windows)]
const ICON_SMALL: usize = 0;
#[cfg(windows)]
const ICON_BIG: usize = 1;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Hinstance;
    fn GetCurrentProcessId() -> u32;
}

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn LoadImageW(
        instance: Hinstance,
        name: *const u16,
        kind: u32,
        width: i32,
        height: i32,
        flags: u32,
    ) -> Hicon;
    fn CreateIcon(
        instance: Hinstance,
        width: i32,
        height: i32,
        planes: u8,
        bits_per_pixel: u8,
        and_mask: *const u8,
        xor_bits: *const u8,
    ) -> Hicon;
    fn SendMessageW(hwnd: Hwnd, message: u32, wparam: usize, lparam: isize) -> isize;
    fn EnumWindows(callback: unsafe extern "system" fn(Hwnd, isize) -> i32, param: isize) -> i32;
    fn GetWindowThreadProcessId(hwnd: Hwnd, process_id: *mut u32) -> u32;
}

#[cfg(test)]
mod tests {
    use super::icon_pixels;

    #[test]
    fn icon_pixels_are_a_square_of_the_requested_size() {
        let pixels = icon_pixels(32).expect("icon png");
        assert_eq!(pixels.len(), 32 * 32 * 4);
    }
}
