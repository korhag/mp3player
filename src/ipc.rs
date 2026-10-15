// ------------------------------------------------------------
// Module: ipc
// Purpose: Single-instance handoff. The first process listens on a
//          local socket. Later processes send their file paths and
//          exit so "Open with" reaches the running window.
//          Paths travel on a tokio channel so the UI task is an
//          ordinary future and can be cancelled when the window
//          closes. A blocking receive would stall process exit.
// ------------------------------------------------------------

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericNamespaced, Listener, ListenerOptions, Stream};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::config;

static INCOMING: OnceLock<Mutex<Option<UnboundedReceiver<Vec<PathBuf>>>>> = OnceLock::new();

// ------------------------------------------------------------
// Function: claim_instance
// Purpose: Binds the socket when this is the first instance.
// Input: paths - Audio files to forward if another instance is up.
// Output: True when this process should open the window.
// ------------------------------------------------------------
pub fn claim_instance(paths: &[PathBuf]) -> bool {
    match bind_server() {
        Ok(receiver) => {
            let _ = INCOMING.set(Mutex::new(Some(receiver)));
            true
        }
        Err(_) => !try_send(paths),
    }
}

// ------------------------------------------------------------
// Function: take_receiver
// Purpose: Hands the socket queue to the interface, once.
// ------------------------------------------------------------
pub fn take_receiver() -> Option<UnboundedReceiver<Vec<PathBuf>>> {
    INCOMING.get()?.lock().ok()?.take()
}

// ------------------------------------------------------------
// Function: try_send
// Purpose: Delivers paths to a running instance.
// Output: True when a running instance accepted the connection.
// ------------------------------------------------------------
pub fn try_send(paths: &[PathBuf]) -> bool {
    let Ok(name) = socket_name() else {
        return false;
    };
    let Ok(mut stream) = Stream::connect(name) else {
        return false;
    };
    let Ok(payload) = serde_json::to_string(paths) else {
        return false;
    };
    writeln!(stream, "{payload}").is_ok()
}

fn bind_server() -> std::io::Result<UnboundedReceiver<Vec<PathBuf>>> {
    let name = socket_name()?;
    let listener = ListenerOptions::new()
        .name(name)
        .try_overwrite(true)
        .create_sync()?;
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    thread::Builder::new()
        .name("mp3player-ipc".into())
        .spawn(move || serve(listener, sender))
        .map_err(std::io::Error::other)?;
    Ok(receiver)
}

fn serve(listener: Listener, sender: UnboundedSender<Vec<PathBuf>>) {
    loop {
        match listener.accept() {
            Ok(stream) => {
                let paths = read_paths(stream);
                if !paths.is_empty() && sender.send(paths).is_err() {
                    break;
                }
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
}

fn read_paths(stream: Stream) -> Vec<PathBuf> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader
        .read_line(&mut line)
        .ok()
        .filter(|count| *count > 0)
        .is_none()
    {
        return Vec::new();
    }
    serde_json::from_str::<Vec<PathBuf>>(line.trim()).unwrap_or_default()
}

fn socket_name() -> std::io::Result<interprocess::local_socket::Name<'static>> {
    config::SOCKET_NAME
        .to_ns_name::<GenericNamespaced>()
        .map_err(std::io::Error::other)
}
