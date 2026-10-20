// ------------------------------------------------------------
// Module: stream
// Purpose: Opens a remote audio URL on its own runtime so the
//          audio thread never waits on the network. The runtime
//          has to stay alive for as long as the reader is playing.
// ------------------------------------------------------------

use std::fs;

use stream_download::storage::temp::TempStorageProvider;
use stream_download::{Settings, StreamDownload};

use crate::config;

pub struct OpenedStream {
    reader: Option<StreamDownload<TempStorageProvider>>,
    runtime: Option<tokio::runtime::Runtime>,
}

impl OpenedStream {
    // --------------------------------------------------------
    // Function: into_parts
    // Purpose: Hands the reader and its runtime to the audio
    //          thread. The reader is dropped before the runtime.
    // --------------------------------------------------------
    pub fn into_parts(mut self) -> (StreamDownload<TempStorageProvider>, tokio::runtime::Runtime) {
        let reader = self.reader.take().expect("stream reader");
        let runtime = self.runtime.take().expect("stream runtime");
        (reader, runtime)
    }
}

impl Drop for OpenedStream {
    fn drop(&mut self) {
        self.reader.take();
        self.runtime.take();
    }
}

// ------------------------------------------------------------
// Function: open_blocking
// Purpose: Downloads into a temp file and returns once the
//          prefetch buffer is ready. Called from a loader thread.
// ------------------------------------------------------------
pub fn open_blocking(url: &str) -> Result<OpenedStream, String> {
    let cache = config::stream_cache_dir();
    fs::create_dir_all(&cache).map_err(|error| error.to_string())?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .thread_name("soundkeep-stream")
        .build()
        .map_err(|error| format!("Could not start the stream loader: {error}"))?;
    let url = url.to_string();
    let reader = runtime.block_on(async move {
        let parsed = reqwest::Url::parse(&url).map_err(|error| error.to_string())?;
        let client = reqwest::Client::builder()
            .user_agent(config::user_agent())
            .build()
            .map_err(|error| error.to_string())?;
        let source = stream_download::http::HttpStream::new(client, parsed)
            .await
            .map_err(|error| format!("Could not open the stream: {error}"))?;
        StreamDownload::from_stream(
            source,
            TempStorageProvider::with_prefix_in("soundkeep", cache),
            Settings::default(),
        )
        .await
        .map_err(|error| format!("Could not open the stream: {error:?}"))
    })?;
    Ok(OpenedStream {
        reader: Some(reader),
        runtime: Some(runtime),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_port_does_not_look_like_audio() {
        match open_blocking("http://127.0.0.1:9/episode.mp3") {
            Err(error) => assert!(!error.is_empty()),
            Ok(_) => panic!("a closed port should not return audio"),
        }
    }
}
