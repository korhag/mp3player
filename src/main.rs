// ------------------------------------------------------------
// Module: main
// Purpose: Process entry. Forwards files to a running window,
//          otherwise starts the player. `--print-data-dir` prints
//          the settings folder for the uninstall scripts.
// ------------------------------------------------------------

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod config;
mod ipc;
mod library;
mod podcast;
mod queue;
mod settings;
mod system;
mod tasks;
mod theme;
mod ui;
mod util;

use std::path::PathBuf;

fn main() -> iced::Result {
    // Move a library saved under the old MP3Player name before any
    // path is printed or settings are loaded.
    config::migrate_legacy_data();
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if args
        .iter()
        .any(|arg| arg == std::path::Path::new("--print-data-dir"))
    {
        print_line(&config::data_dir().display().to_string());
        return Ok(());
    }
    let paths = library::collect_audio_paths(&args);
    if !ipc::claim_instance(&paths) {
        return Ok(());
    }
    app::launch(paths)
}

// ------------------------------------------------------------
// Function: print_line
// Purpose: Writes one line. A Windows GUI build has no console
//          unless we attach to the parent terminal first.
// ------------------------------------------------------------
fn print_line(text: &str) {
    #[cfg(windows)]
    {
        write_console_line(text);
    }
    #[cfg(not(windows))]
    {
        println!("{text}");
    }
}

#[cfg(windows)]
fn write_console_line(text: &str) {
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;
    const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5;
    let mut bytes = text.as_bytes().to_vec();
    bytes.extend_from_slice(b"\r\n");
    unsafe {
        // Prefer an inherited pipe (PowerShell redirection) before
        // attaching to a visible console.
        if write_handle(GetStdHandle(STD_OUTPUT_HANDLE), &bytes) {
            return;
        }
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        let _ = write_handle(GetStdHandle(STD_OUTPUT_HANDLE), &bytes);
    }
}

#[cfg(windows)]
unsafe fn write_handle(handle: *mut core::ffi::c_void, bytes: &[u8]) -> bool {
    if handle.is_null() || handle == usize::MAX as *mut core::ffi::c_void {
        return false;
    }
    let mut written = 0u32;
    let ok = unsafe {
        WriteFile(
            handle,
            bytes.as_ptr(),
            bytes.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        )
    };
    ok != 0 && written as usize == bytes.len()
}

#[cfg(windows)]
unsafe extern "system" {
    fn AttachConsole(process_id: u32) -> i32;
    fn GetStdHandle(std_handle: u32) -> *mut core::ffi::c_void;
    fn WriteFile(
        file: *mut core::ffi::c_void,
        buffer: *const u8,
        number_of_bytes_to_write: u32,
        number_of_bytes_written: *mut u32,
        overlapped: *mut core::ffi::c_void,
    ) -> i32;
}
