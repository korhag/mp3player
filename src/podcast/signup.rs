// ------------------------------------------------------------
// Module: signup
// Purpose: Opens Podcast Index's signup page so the user can
//          request their own API key. The key arrives by email.
//          Their terms forbid embedding a shared secret in the app.
// ------------------------------------------------------------

use crate::system;

pub const SIGNUP_URL: &str = "https://api.podcastindex.org/signup";

// The in-app window is Windows-only. These stay out of the Linux build
// so a release compile does not warn about unused imports.
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::mpsc;
#[cfg(windows)]
use std::time::Duration;

#[cfg(windows)]
static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignupPlace {
    /// Opened in the in-app window. Constructed on Windows only.
    #[cfg_attr(not(windows), allow(dead_code))]
    Window,
    Browser,
}

// ------------------------------------------------------------
// Function: open_signup
// Purpose: Shows the signup page in a window on Windows, and in
//          the browser everywhere else or if that window fails.
// ------------------------------------------------------------
pub fn open_signup() -> Result<SignupPlace, String> {
    #[cfg(windows)]
    {
        match open_window() {
            Ok(()) => Ok(SignupPlace::Window),
            Err(error) => {
                system::open_url(SIGNUP_URL).map_err(|browser| {
                    format!("Could not open signup: {error}. Browser: {browser}")
                })?;
                Ok(SignupPlace::Browser)
            }
        }
    }
    #[cfg(not(windows))]
    {
        system::open_url(SIGNUP_URL).map_err(|error| error.to_string())?;
        Ok(SignupPlace::Browser)
    }
}

#[cfg(windows)]
fn open_window() -> Result<(), String> {
    if WINDOW_OPEN
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }
    let (tx, rx) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("mp3player-signup".into())
        .spawn(move || run_signup_window(tx));
    if spawned.is_err() {
        WINDOW_OPEN.store(false, Ordering::SeqCst);
        return Err("Could not start the signup window.".to_string());
    }
    match rx.recv_timeout(Duration::from_secs(20)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            WINDOW_OPEN.store(false, Ordering::SeqCst);
            Err(error)
        }
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(()),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            WINDOW_OPEN.store(false, Ordering::SeqCst);
            Err("The signup window closed before it finished opening.".to_string())
        }
    }
}

#[cfg(windows)]
fn run_signup_window(tx: mpsc::Sender<Result<(), String>>) {
    if let Err(error) = show_signup(&tx) {
        let _ = tx.send(Err(error));
        WINDOW_OPEN.store(false, Ordering::SeqCst);
    }
}

#[cfg(windows)]
fn show_signup(ready: &mpsc::Sender<Result<(), String>>) -> Result<(), String> {
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::platform::run_return::EventLoopExtRunReturn;
    use tao::window::WindowBuilder;
    use wry::WebViewBuilder;

    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Request a Podcast Index key")
        .with_inner_size(LogicalSize::new(460.0, 780.0))
        .build(&event_loop)
        .map_err(|error| error.to_string())?;
    let webview = WebViewBuilder::new()
        .with_url(SIGNUP_URL)
        .build(&window)
        .map_err(|error| error.to_string())?;
    let _ = ready.send(Ok(()));
    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
    drop(webview);
    WINDOW_OPEN.store(false, Ordering::SeqCst);
    Ok(())
}
