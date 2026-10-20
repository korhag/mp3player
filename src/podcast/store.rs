// ------------------------------------------------------------
// Module: store
// Purpose: Caches favorite shows and their last episode lists so
//          the tab still opens with no network.
// ------------------------------------------------------------

use std::fs;
use std::path::Path;

use crate::config;
use crate::podcast::Show;

pub fn load_favorites() -> Vec<Show> {
    load_from(&config::favorites_path())
}

pub fn save_favorites(shows: &[Show]) -> std::io::Result<()> {
    save_to(&config::favorites_path(), shows)
}

fn load_from(path: &Path) -> Vec<Show> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(mut shows) = serde_json::from_str::<Vec<Show>>(&text) else {
        return Vec::new();
    };
    for show in &mut shows {
        for episode in &mut show.episodes {
            if episode
                .local_path
                .as_ref()
                .is_some_and(|path| !path.is_file())
            {
                episode.local_path = None;
            }
        }
    }
    shows
}

fn save_to(path: &Path, shows: &[Show]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(shows).map_err(std::io::Error::other)?;
    fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::podcast::Episode;

    #[test]
    fn favorites_round_trip() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "soundkeep-favorites-{}-{nanos}.json",
            std::process::id()
        ));
        let shows = vec![Show {
            feed_url: "https://example.com/feed.xml".to_string(),
            title: "Sample".to_string(),
            author: "Host".to_string(),
            image_url: None,
            description: String::new(),
            episodes: vec![Episode {
                guid: "ep-1".to_string(),
                title: "One".to_string(),
                audio_url: "https://example.com/one.mp3".to_string(),
                duration_secs: 90,
                published_secs: None,
                local_path: None,
            }],
        }];
        save_to(&path, &shows).unwrap();
        assert_eq!(load_from(&path), shows);
        let _ = fs::remove_file(path);
    }
}
