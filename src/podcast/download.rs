// ------------------------------------------------------------
// Module: download
// Purpose: Saves an episode to disk. A `.part` file is renamed
//          only after the whole download finishes.
// ------------------------------------------------------------

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use iced::futures::StreamExt;
use iced::{Task, stream};

use crate::app::{Message, PodcastMessage};
use crate::config;
use crate::library::sanitize_name;
use crate::podcast::search::http_client;
use crate::podcast::{Episode, PodcastState};

// ------------------------------------------------------------
// Function: episode_path
// Purpose: Destination for one episode under the show's folder.
// ------------------------------------------------------------
pub fn episode_path(show_title: &str, episode: &Episode) -> PathBuf {
    let extension = audio_extension(&episode.audio_url);
    let name = sanitize_name(&format!("{}-{}", episode.title, short_id(&episode.guid)));
    config::podcast_download_dir()
        .join(sanitize_name(show_title))
        .join(format!("{name}.{extension}"))
}

pub fn remove_download(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

// ------------------------------------------------------------
// Function: download_task
// Purpose: Streams the file and reports progress to the window.
// ------------------------------------------------------------
pub fn download_task(feed_url: String, episode: Episode, destination: PathBuf) -> Task<Message> {
    let key = PodcastState::progress_key(&feed_url, &episode.guid);
    let guid = episode.guid.clone();
    let url = episode.audio_url.clone();
    Task::stream(stream::channel(16, async move |mut sender| {
        let result = fetch_to(&url, &destination, &key, &mut sender).await;
        let _ = sender.try_send(PodcastMessage::DownloadFinished {
            feed_url,
            guid,
            result,
        });
    }))
    .map(Message::Podcast)
}

async fn fetch_to(
    url: &str,
    destination: &Path,
    progress_key: &str,
    sender: &mut iced::futures::channel::mpsc::Sender<PodcastMessage>,
) -> Result<PathBuf, String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut part = destination.as_os_str().to_os_string();
    part.push(".part");
    let part = PathBuf::from(part);
    let client = http_client()?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Download failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Download failed ({})", response.status()));
    }
    let total = response.content_length().unwrap_or(0);
    let mut file = fs::File::create(&part).map_err(|error| error.to_string())?;
    let mut body = response.bytes_stream();
    let mut received = 0u64;
    let mut last_percent = 0u8;
    while let Some(chunk) = body.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                let _ = fs::remove_file(&part);
                return Err(format!("Download failed: {error}"));
            }
        };
        if file.write_all(&chunk).is_err() {
            let _ = fs::remove_file(&part);
            return Err("Could not write the download.".to_string());
        }
        received += chunk.len() as u64;
        if total > 0 {
            let percent = ((received.saturating_mul(100)) / total) as u8;
            if percent != last_percent {
                last_percent = percent;
                let _ = sender.try_send(PodcastMessage::DownloadProgress {
                    key: progress_key.to_string(),
                    fraction: percent as f32 / 100.0,
                });
            }
        }
    }
    file.sync_all().ok();
    drop(file);
    if destination.exists() {
        let _ = fs::remove_file(destination);
    }
    fs::rename(&part, destination).map_err(|error| {
        let _ = fs::remove_file(&part);
        format!("Could not finish the download: {error}")
    })?;
    Ok(destination.to_path_buf())
}

fn audio_extension(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("mp3");
    let cleaned: String = extension
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(5)
        .collect();
    if cleaned.is_empty() {
        "mp3".to_string()
    } else {
        cleaned.to_ascii_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::fetch_to;

    #[test]
    fn download_renames_the_part_file_only_when_finished() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = b"episode-bytes";
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0u8; 2048];
            let _ = std::io::Read::read(&mut stream, &mut buffer);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            std::io::Write::write_all(&mut stream, header.as_bytes()).unwrap();
            std::io::Write::write_all(&mut stream, body).unwrap();
        });
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("mp3player-download-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let destination = dir.join("episode.mp3");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (mut sender, _receiver) = iced::futures::channel::mpsc::channel(8);
        let result = runtime.block_on(fetch_to(
            &format!("http://127.0.0.1:{port}/episode.mp3"),
            &destination,
            "show\nep",
            &mut sender,
        ));
        assert_eq!(result.unwrap(), destination);
        assert_eq!(fs::read(&destination).unwrap(), body);
        let mut part = destination.as_os_str().to_os_string();
        part.push(".part");
        assert!(!PathBuf::from(part).exists());
        let _ = fs::remove_dir_all(dir);
    }
}

fn short_id(guid: &str) -> String {
    let id: String = guid
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(8)
        .collect();
    if id.is_empty() {
        "episode".to_string()
    } else {
        id
    }
}
