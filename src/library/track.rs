// ------------------------------------------------------------
// Module: track
// Purpose: One playable file plus the metadata shown in the queue.
// ------------------------------------------------------------

use std::path::{Path, PathBuf};
use std::time::Duration;

use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::probe::Probe;

use crate::util::format_time;

/// Largest embedded image kept in memory for the now-playing artwork.
const MAX_COVER_BYTES: usize = 1_500_000;

// ------------------------------------------------------------
// Struct: Track
// Purpose: Path, display metadata, and optional embedded cover art.
// ------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
    pub cover: Option<Vec<u8>>,
    /// The file could not be opened. The row stays so it can be relocated.
    pub missing: bool,
    /// Remote audio. A downloaded copy in `path` is preferred when it exists.
    pub stream_url: Option<String>,
}

impl Track {
    // --------------------------------------------------------
    // Function: load
    // Purpose: Reads tags when possible. A missing tag still returns
    //          a track so the file can be played.
    // Input: path - Audio file to inspect.
    // Output: Track - Filename is used when a title is absent.
    // --------------------------------------------------------
    pub fn load(path: &Path) -> Self {
        let mut track = read_tags(path).unwrap_or_else(|_| Self::from_path(path));
        track.missing = !path.is_file();
        track
    }

    // --------------------------------------------------------
    // Function: from_path
    // Purpose: Builds a track from the filename alone.
    // --------------------------------------------------------
    pub fn from_path(path: &Path) -> Self {
        Self {
            title: path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Unknown title")
                .to_string(),
            path: path.to_path_buf(),
            artist: String::new(),
            album: String::new(),
            duration: Duration::ZERO,
            cover: None,
            missing: false,
            stream_url: None,
        }
    }

    // --------------------------------------------------------
    // Function: from_stream
    // Purpose: A podcast episode that plays from a URL until it
    //          is downloaded.
    // --------------------------------------------------------
    pub fn from_stream(title: &str, artist: &str, url: &str, duration: Duration) -> Self {
        Self {
            path: PathBuf::new(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: String::new(),
            duration,
            cover: None,
            missing: false,
            stream_url: Some(url.to_string()),
        }
    }

    // --------------------------------------------------------
    // Function: plays_from_file
    // Purpose: True when a local file can be opened right now.
    // --------------------------------------------------------
    pub fn plays_from_file(&self) -> bool {
        self.path.is_file()
    }

    // --------------------------------------------------------
    // Function: is_persistable
    // Purpose: Session queues and playlists store local files.
    //          Stream-only episodes are skipped until downloaded.
    // --------------------------------------------------------
    pub fn is_persistable(&self) -> bool {
        if self.stream_url.is_some() {
            self.path.is_file()
        } else {
            !self.path.as_os_str().is_empty()
        }
    }

    // --------------------------------------------------------
    // Function: artist_label
    // Purpose: Text for the artist line, including the album.
    // --------------------------------------------------------
    pub fn artist_label(&self) -> String {
        match (self.artist.is_empty(), self.album.is_empty()) {
            (true, true) => "Unknown artist".to_string(),
            (false, true) => self.artist.clone(),
            (true, false) => self.album.clone(),
            (false, false) => format!("{} — {}", self.artist, self.album),
        }
    }

    // --------------------------------------------------------
    // Function: duration_label
    // Purpose: Compact duration for a queue row.
    // --------------------------------------------------------
    pub fn duration_label(&self) -> String {
        format_time(self.duration)
    }

    #[cfg(test)]
    pub fn placeholder(name: &str) -> Self {
        Self::from_path(Path::new(name))
    }
}

// ------------------------------------------------------------
// Function: read_tags
// Purpose: Pulls title, artist, album, duration, and cover art.
// ------------------------------------------------------------
fn read_tags(path: &Path) -> Result<Track, String> {
    let tagged = Probe::open(path)
        .map_err(|error| error.to_string())?
        .read()
        .map_err(|error| error.to_string())?;
    let duration = tagged.properties().duration();
    let mut track = Track::from_path(path);
    track.duration = duration;
    if let Some(tag) = tagged.primary_tag() {
        if let Some(title) = tag.title() {
            let title = title.trim();
            if !title.is_empty() {
                track.title = title.to_string();
            }
        }
        track.artist = tag.artist().unwrap_or_default().trim().to_string();
        track.album = tag.album().unwrap_or_default().trim().to_string();
        track.cover = cover_bytes(tag);
    }
    Ok(track)
}

// ------------------------------------------------------------
// Function: cover_bytes
// Purpose: Prefers front-cover art and ignores oversized images.
// ------------------------------------------------------------
fn cover_bytes(tag: &lofty::tag::Tag) -> Option<Vec<u8>> {
    let pictures = tag.pictures();
    let picture = pictures
        .iter()
        .find(|picture| picture.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures.first())?;
    let bytes = picture.data();
    if bytes.is_empty() || bytes.len() > MAX_COVER_BYTES {
        return None;
    }
    Some(bytes.to_vec())
}
