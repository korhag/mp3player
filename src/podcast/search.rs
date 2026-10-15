// ------------------------------------------------------------
// Module: search
// Purpose: Podcast Index directory search. The free API key is
//          read from settings, unless the environment overrides it.
// ------------------------------------------------------------

use crate::config;
use crate::podcast::{Episode, KeyStatus, Show};

const SEARCH_URL: &str = "https://api.podcastindex.org/api/1.0/search/byterm";
const STATS_URL: &str = "https://api.podcastindex.org/api/1.0/stats/current";
const EPISODES_URL: &str = "https://api.podcastindex.org/api/1.0/episodes/byfeedurl";

// ------------------------------------------------------------
// Function: credentials
// Purpose: Environment variables win over the saved key so a
//          dev machine can search without writing settings.
// ------------------------------------------------------------
pub fn credentials(stored_key: &str, stored_secret: &str) -> Option<(String, String)> {
    let key = std::env::var("PODCASTINDEX_API_KEY").unwrap_or_else(|_| stored_key.to_string());
    let secret =
        std::env::var("PODCASTINDEX_API_SECRET").unwrap_or_else(|_| stored_secret.to_string());
    let key = key.trim().to_string();
    let secret = secret.trim().to_string();
    if key.is_empty() || secret.is_empty() {
        None
    } else {
        Some((key, secret))
    }
}

pub fn using_environment() -> bool {
    std::env::var("PODCASTINDEX_API_KEY")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
        && std::env::var("PODCASTINDEX_API_SECRET")
            .ok()
            .is_some_and(|value| !value.trim().is_empty())
}

// ------------------------------------------------------------
// Function: authorization
// Purpose: Podcast Index wants sha1 hex of key + secret + unix time.
// ------------------------------------------------------------
pub fn authorization(key: &str, secret: &str, unix_secs: &str) -> String {
    let digest = sha1_smol::Sha1::from(format!("{key}{secret}{unix_secs}")).digest();
    hex_encode(&digest.bytes())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(config::user_agent())
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|error| error.to_string())
}

// ------------------------------------------------------------
// Function: signed_get
// Purpose: One authenticated GET. Search, the key check, and
//          older episodes all sign the request the same way.
// ------------------------------------------------------------
async fn signed_get(
    url: reqwest::Url,
    key: &str,
    secret: &str,
) -> Result<reqwest::Response, String> {
    let date = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs()
        .to_string();
    let client = http_client()?;
    client
        .get(url)
        .header("User-Agent", config::user_agent())
        .header("X-Auth-Key", key)
        .header("X-Auth-Date", &date)
        .header("Authorization", authorization(key, secret, &date))
        .send()
        .await
        .map_err(|error| format!("Podcast Index request failed: {error}"))
}

fn rejected(status: reqwest::StatusCode) -> bool {
    status.as_u16() == 401 || status.as_u16() == 403
}

// ------------------------------------------------------------
// Function: check_key
// Purpose: Asks Podcast Index whether this key is accepted.
//          A small stats call is enough; it does not list shows.
// ------------------------------------------------------------
pub async fn check_key(key: String, secret: String) -> KeyStatus {
    let url = match reqwest::Url::parse(STATS_URL) {
        Ok(url) => url,
        Err(error) => return KeyStatus::Unreachable(error.to_string()),
    };
    match signed_get(url, &key, &secret).await {
        Ok(response) => {
            let status = response.status();
            if rejected(status) {
                KeyStatus::Rejected
            } else if status.is_success() {
                KeyStatus::Valid
            } else {
                KeyStatus::Unreachable(format!("Podcast Index returned {status}"))
            }
        }
        Err(error) => KeyStatus::Unreachable(error),
    }
}

// ------------------------------------------------------------
// Function: search
// Purpose: Looks up shows by term. Episodes are not included;
//          those always come from the show's RSS feed.
// ------------------------------------------------------------
pub async fn search(query: String, key: String, secret: String) -> Result<Vec<Show>, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("Type a show name to search.".to_string());
    }
    let url = reqwest::Url::parse_with_params(SEARCH_URL, &[("q", query.as_str())])
        .map_err(|error| error.to_string())?;
    let response = signed_get(url, &key, &secret).await?;
    let status = response.status();
    if rejected(status) {
        return Err(format!("Podcast Index rejected the API key ({status})."));
    }
    if !status.is_success() {
        return Err(format!("Search failed ({status})"));
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|error| format!("Search returned unreadable data: {error}"))?;
    let Some(feeds) = body.get("feeds").and_then(|value| value.as_array()) else {
        let reason = body
            .get("description")
            .and_then(|value| value.as_str())
            .unwrap_or("Search returned no shows.");
        return Err(reason.to_string());
    };
    let mut shows = Vec::new();
    for feed in feeds.iter().take(25) {
        let Some(feed_url) = feed.get("url").and_then(|value| value.as_str()) else {
            continue;
        };
        if feed_url.is_empty() {
            continue;
        }
        shows.push(Show {
            feed_url: feed_url.to_string(),
            title: text_field(feed, "title").unwrap_or_else(|| "Untitled show".to_string()),
            author: text_field(feed, "author").unwrap_or_default(),
            image_url: text_field(feed, "image").or_else(|| text_field(feed, "artwork")),
            description: text_field(feed, "description")
                .map(|text| crate::podcast::plain_text(&text))
                .unwrap_or_default(),
            episodes: Vec::new(),
        });
    }
    Ok(shows)
}

// ------------------------------------------------------------
// Function: older_episodes
// Purpose: Asks Podcast Index for episodes past the short RSS
//          list. `max` is the API's upper bound.
// ------------------------------------------------------------
pub async fn older_episodes(
    feed_url: String,
    key: String,
    secret: String,
) -> Result<Vec<Episode>, String> {
    let url = reqwest::Url::parse_with_params(
        EPISODES_URL,
        &[("url", feed_url.as_str()), ("max", "1000")],
    )
    .map_err(|error| error.to_string())?;
    let response = signed_get(url, &key, &secret).await?;
    let status = response.status();
    if rejected(status) {
        return Err(format!("Podcast Index rejected the API key ({status})."));
    }
    if !status.is_success() {
        return Err(format!("Could not load older episodes ({status})"));
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|error| format!("Could not read older episodes: {error}"))?;
    let Some(items) = body.get("items").and_then(|value| value.as_array()) else {
        let reason = body
            .get("description")
            .and_then(|value| value.as_str())
            .unwrap_or("Podcast Index returned no episodes.");
        return Err(reason.to_string());
    };
    Ok(items.iter().filter_map(episode_from_index).collect())
}

fn episode_from_index(item: &serde_json::Value) -> Option<Episode> {
    let audio_url = text_field(item, "enclosureUrl")?;
    let guid = text_field(item, "guid").unwrap_or_else(|| audio_url.clone());
    let title = text_field(item, "title").unwrap_or_else(|| "Untitled episode".to_string());
    let duration_secs = item
        .get("duration")
        .and_then(|value| {
            value.as_u64().or_else(|| {
                value
                    .as_i64()
                    .filter(|secs| *secs >= 0)
                    .map(|secs| secs as u64)
            })
        })
        .unwrap_or(0);
    let published_secs = item
        .get("datePublished")
        .and_then(|value| value.as_i64())
        .filter(|secs| *secs > 0);
    Some(Episode {
        guid,
        title,
        audio_url,
        duration_secs,
        published_secs,
        local_path: None,
    })
}

fn text_field(value: &serde_json::Value, name: &str) -> Option<String> {
    let text = value.get(name)?.as_str()?.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_matches_a_known_sha1() {
        assert_eq!(
            authorization("key", "secret", "1700000000"),
            "abaf71c02050c31e4d4e6b08c1625173af0445ba"
        );
    }

    #[test]
    fn maps_a_podcast_index_episode() {
        let item = serde_json::json!({
            "title": "Older episode",
            "guid": "ep-old",
            "enclosureUrl": "https://example.com/old.mp3",
            "duration": 90,
            "datePublished": 1_787_616_000
        });
        let episode = episode_from_index(&item).unwrap();
        assert_eq!(episode.title, "Older episode");
        assert_eq!(episode.guid, "ep-old");
        assert_eq!(episode.audio_url, "https://example.com/old.mp3");
        assert_eq!(episode.duration_secs, 90);
        assert_eq!(episode.published_secs, Some(1_787_616_000));
        assert!(episode.local_path.is_none());
    }
}
