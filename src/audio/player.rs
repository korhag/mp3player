// ------------------------------------------------------------
// Module: player
// Purpose: Wraps a rodio Player. Commands go in through a channel.
//          The interface reads a snapshot for position and state.
// ------------------------------------------------------------

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use rodio::decoder::DecoderBuilder;
use rodio::{Decoder, DeviceSinkBuilder, Player};

use super::stream::{self, OpenedStream};

// ------------------------------------------------------------
// Struct: Snapshot
// Purpose: Playback state safe to read from the interface thread.
// ------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub generation: u64,
    pub position: Duration,
    pub paused: bool,
    pub empty: bool,
    pub active: bool,
    /// A stream request is in flight and audio has not started.
    pub loading: bool,
    pub error: Option<String>,
}

// ------------------------------------------------------------
// Enum: Phase
// Purpose: What the interface should say about the current track.
//          Loading wins, because a stream that is still opening
//          reports itself as paused until the buffer is ready.
// ------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Loading,
    Playing,
    Paused,
    Stopped,
}

impl Snapshot {
    pub fn phase(&self) -> Phase {
        if self.loading {
            Phase::Loading
        } else if self.active && !self.paused {
            Phase::Playing
        } else if self.active && self.paused {
            Phase::Paused
        } else {
            Phase::Stopped
        }
    }
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            generation: 0,
            position: Duration::ZERO,
            paused: true,
            empty: true,
            active: false,
            loading: false,
            error: None,
        }
    }
}

enum Command {
    Play(PathBuf),
    PlayStream(String),
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    Volume(f32),
    Shutdown,
}

struct PendingStream {
    token: u64,
    rx: Receiver<Result<OpenedStream, String>>,
}

// ------------------------------------------------------------
// Struct: AudioHandle
// Purpose: The interface's handle to the audio thread.
// ------------------------------------------------------------
pub struct AudioHandle {
    tx: Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    thread: Option<JoinHandle<()>>,
}

impl AudioHandle {
    // --------------------------------------------------------
    // Function: start
    // Purpose: Opens the default output device and starts the
    //          audio thread. The window still opens if no device
    //          is available; playback then reports an error.
    // Input: volume - Linear gain from 0.0 to 1.0.
    // --------------------------------------------------------
    pub fn start(volume: f32) -> Self {
        let (tx, rx) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let shared = Arc::clone(&snapshot);
        let thread = std::thread::Builder::new()
            .name("mp3player-audio".into())
            .spawn(move || audio_thread(rx, shared, volume))
            .ok();
        if thread.is_none() {
            set_error(&snapshot, "Could not start the audio thread.");
        }
        Self {
            tx,
            snapshot,
            thread,
        }
    }

    pub fn play(&self, path: PathBuf) {
        let _ = self.tx.send(Command::Play(path));
    }

    pub fn play_stream(&self, url: String) {
        let _ = self.tx.send(Command::PlayStream(url));
    }

    pub fn pause(&self) {
        let _ = self.tx.send(Command::Pause);
    }

    pub fn resume(&self) {
        let _ = self.tx.send(Command::Resume);
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Command::Stop);
    }

    pub fn seek(&self, position: Duration) {
        let _ = self.tx.send(Command::Seek(position));
    }

    pub fn set_volume(&self, volume: f32) {
        let _ = self.tx.send(Command::Volume(volume.clamp(0.0, 1.0)));
    }

    // --------------------------------------------------------
    // Function: snapshot
    // Purpose: Copies the latest playback state.
    // --------------------------------------------------------
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
}

impl Drop for AudioHandle {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn audio_thread(rx: Receiver<Command>, snapshot: Arc<Mutex<Snapshot>>, mut volume: f32) {
    let output = match DeviceSinkBuilder::open_default_sink() {
        Ok(output) => output,
        Err(error) => {
            set_error(
                &snapshot,
                &format!("No audio output device is available: {error}"),
            );
            return;
        }
    };
    let player = Player::connect_new(output.mixer());
    player.set_volume(volume);
    let mut generation = 0u64;
    let mut play_token = 0u64;
    let mut active = false;
    let mut error = None;
    let mut pending: Option<PendingStream> = None;
    // Kept until the next track so the stream's download task stays alive.
    let mut stream_runtime: Option<tokio::runtime::Runtime> = None;

    loop {
        let command = match rx.recv_timeout(Duration::from_millis(80)) {
            Ok(command) => Some(command),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        let mut seek_position = None;
        if let Some(command) = command {
            match command {
                Command::Shutdown => break,
                Command::Play(path) => {
                    play_token += 1;
                    pending = None;
                    finish_stream(&player, &mut stream_runtime);
                    generation += 1;
                    match load_source(&player, &path, volume) {
                        Ok(()) => {
                            active = true;
                            error = None;
                        }
                        Err(message) => {
                            player.stop();
                            active = false;
                            error = Some(message);
                        }
                    }
                }
                Command::PlayStream(url) => {
                    play_token += 1;
                    let token = play_token;
                    finish_stream(&player, &mut stream_runtime);
                    active = false;
                    error = None;
                    pending = Some(spawn_stream(url, token));
                }
                Command::Pause => {
                    player.pause();
                }
                Command::Resume => {
                    if active {
                        player.play();
                    }
                }
                Command::Stop => {
                    play_token += 1;
                    pending = None;
                    finish_stream(&player, &mut stream_runtime);
                    player.stop();
                    active = false;
                    error = None;
                }
                Command::Seek(position) => {
                    if player.try_seek(position).is_ok() {
                        seek_position = Some(position);
                    }
                }
                Command::Volume(value) => {
                    volume = value;
                    player.set_volume(volume);
                }
            }
        }

        if let Some(arrived) = take_ready_stream(&mut pending, play_token) {
            match arrived {
                Ok(opened) => match start_stream(&player, opened, volume) {
                    Ok(runtime) => {
                        stream_runtime = Some(runtime);
                        generation += 1;
                        active = true;
                        error = None;
                    }
                    Err(message) => {
                        generation += 1;
                        active = false;
                        error = Some(message);
                    }
                },
                Err(message) => {
                    generation += 1;
                    active = false;
                    error = Some(message);
                }
            }
        }

        publish(
            &snapshot,
            &player,
            generation,
            active,
            pending.is_some(),
            error.clone(),
            seek_position,
        );
    }
}

fn finish_stream(player: &Player, runtime: &mut Option<tokio::runtime::Runtime>) {
    player.clear();
    runtime.take();
}

fn spawn_stream(url: String, token: u64) -> PendingStream {
    let (tx, rx) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("mp3player-stream-loader".into())
        .spawn(move || {
            let _ = tx.send(stream::open_blocking(&url));
        });
    if spawned.is_err() {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(Err("Could not start the stream loader.".to_string()));
        return PendingStream { token, rx };
    }
    PendingStream { token, rx }
}

// A stream that finishes after the listener has moved on is dropped here.
fn take_ready_stream(
    pending: &mut Option<PendingStream>,
    play_token: u64,
) -> Option<Result<OpenedStream, String>> {
    let current = pending.as_ref()?;
    if current.token != play_token {
        pending.take();
        return None;
    }
    match current.rx.try_recv() {
        Ok(result) => {
            pending.take();
            Some(result)
        }
        Err(mpsc::TryRecvError::Empty) => None,
        Err(mpsc::TryRecvError::Disconnected) => {
            pending.take();
            Some(Err("The stream loader stopped.".to_string()))
        }
    }
}

fn start_stream(
    player: &Player,
    opened: OpenedStream,
    volume: f32,
) -> Result<tokio::runtime::Runtime, String> {
    let (reader, runtime) = opened.into_parts();
    let byte_len = reader.content_length().filter(|len| *len > 0);
    let mut builder = DecoderBuilder::new().with_data(reader);
    if let Some(byte_len) = byte_len {
        builder = builder.with_byte_len(byte_len);
    }
    let decoder = match builder.build() {
        Ok(decoder) => decoder,
        Err(error) => {
            drop(runtime);
            return Err(format!("Could not play the stream: {error}"));
        }
    };
    player.clear();
    player.append(decoder);
    player.set_volume(volume);
    player.play();
    Ok(runtime)
}

fn load_source(player: &Player, path: &std::path::Path, volume: f32) -> Result<(), String> {
    let file =
        File::open(path).map_err(|error| format!("Could not open {}: {error}", path.display()))?;
    let decoder = Decoder::new(BufReader::new(file))
        .map_err(|error| format!("Could not play {}: {error}", path.display()))?;
    player.clear();
    player.append(decoder);
    player.set_volume(volume);
    player.play();
    Ok(())
}

fn publish(
    snapshot: &Mutex<Snapshot>,
    player: &Player,
    generation: u64,
    active: bool,
    loading: bool,
    error: Option<String>,
    seek_position: Option<Duration>,
) {
    let mut current = snapshot.lock().unwrap_or_else(|poison| poison.into_inner());
    current.generation = generation;
    current.position = seek_position.unwrap_or_else(|| {
        if active {
            player.get_pos()
        } else {
            Duration::ZERO
        }
    });
    current.paused = !active || player.is_paused();
    current.empty = !active || player.empty();
    current.active = active;
    current.loading = loading;
    current.error = error;
}

fn set_error(snapshot: &Mutex<Snapshot>, message: &str) {
    let mut current = snapshot.lock().unwrap_or_else(|poison| poison.into_inner());
    current.error = Some(message.to_string());
    current.active = false;
    current.empty = true;
    current.paused = true;
    current.loading = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(loading: bool, active: bool, paused: bool) -> Snapshot {
        Snapshot {
            loading,
            active,
            paused,
            ..Snapshot::default()
        }
    }

    #[test]
    fn phase_covers_loading_playing_paused_and_stopped() {
        assert_eq!(snapshot(true, false, true).phase(), Phase::Loading);
        assert_eq!(snapshot(true, true, false).phase(), Phase::Loading);
        assert_eq!(snapshot(false, true, false).phase(), Phase::Playing);
        assert_eq!(snapshot(false, true, true).phase(), Phase::Paused);
        assert_eq!(snapshot(false, false, true).phase(), Phase::Stopped);
    }
}
