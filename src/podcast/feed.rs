// ------------------------------------------------------------
// Module: feed
// Purpose: Reads a podcast RSS feed, including iTunes duration.
//          Search hits and pasted URLs both land here.
// ------------------------------------------------------------

use crate::podcast::search::http_client;
use crate::podcast::{Episode, Show};

// ------------------------------------------------------------
// Function: fetch_show
// Purpose: Downloads a feed and parses it. Failure leaves any
//          cached episodes untouched at the call site.
// ------------------------------------------------------------
pub async fn fetch_show(feed_url: String) -> Result<Show, String> {
    let client = http_client()?;
    let response = client
        .get(&feed_url)
        .send()
        .await
        .map_err(|error| format!("Could not read the feed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Could not read the feed ({})", response.status()));
    }
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read the feed: {error}"))?;
    parse_rss(&body, &feed_url)
}

// ------------------------------------------------------------
// Function: parse_rss
// Purpose: Pulls the show title and episodes that have audio.
// ------------------------------------------------------------
pub fn parse_rss(xml: &str, feed_url: &str) -> Result<Show, String> {
    let channel = rss::Channel::read_from(xml.as_bytes()).map_err(|error| error.to_string())?;
    let itunes = channel.itunes_ext();
    let title = channel.title.trim();
    let title = if title.is_empty() {
        "Untitled show".to_string()
    } else {
        title.to_string()
    };
    let author = itunes
        .and_then(|ext| ext.author())
        .map(str::trim)
        .filter(|author| !author.is_empty())
        .unwrap_or("")
        .to_string();
    let description = crate::podcast::plain_text(channel.description());
    let image_url = itunes
        .and_then(|ext| ext.image())
        .map(str::trim)
        .filter(|image| !image.is_empty())
        .map(str::to_string);
    let mut episodes = Vec::new();
    for item in channel.items() {
        let Some(enclosure) = item.enclosure() else {
            continue;
        };
        let audio_url = enclosure.url().trim();
        if audio_url.is_empty() {
            continue;
        }
        let guid = item
            .guid()
            .map(|guid| guid.value().trim().to_string())
            .filter(|guid| !guid.is_empty())
            .unwrap_or_else(|| audio_url.to_string());
        let title = item
            .title()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .unwrap_or("Untitled episode")
            .to_string();
        let duration_secs = item
            .itunes_ext()
            .and_then(|ext| ext.duration())
            .map(parse_duration)
            .unwrap_or(0);
        episodes.push(Episode {
            guid,
            title,
            audio_url: audio_url.to_string(),
            duration_secs,
            published_secs: published_secs(item),
            local_path: None,
        });
    }
    episodes = crate::podcast::merge_episodes(&episodes, &[]);
    Ok(Show {
        feed_url: feed_url.to_string(),
        title,
        author,
        image_url,
        description,
        episodes,
    })
}

// ------------------------------------------------------------
// Function: fetch_artwork
// Purpose: Downloads a show image and decodes it. The window
//          build has no image codecs, so pixels are decoded here.
// ------------------------------------------------------------
pub async fn fetch_artwork(url: String) -> Result<(u32, u32, Vec<u8>), String> {
    let client = http_client()?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| format!("Could not load artwork: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Could not load artwork ({})", response.status()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Could not load artwork: {error}"))?;
    if bytes.len() > 5_000_000 {
        return Err("Artwork is too large.".to_string());
    }
    let image = ::image::load_from_memory(&bytes)
        .map_err(|error| format!("Could not read artwork: {error}"))?
        .thumbnail(256, 256)
        .to_rgba8();
    let (width, height) = image.dimensions();
    Ok((width, height, image.into_raw()))
}

fn published_secs(item: &rss::Item) -> Option<i64> {
    let text = item.pub_date()?.trim();
    if text.is_empty() {
        return None;
    }
    chrono::DateTime::parse_from_rfc2822(text)
        .ok()
        .map(|published| published.timestamp())
}

fn parse_duration(value: &str) -> u64 {
    let value = value.trim();
    if value.is_empty() {
        return 0;
    }
    if let Ok(seconds) = value.parse::<u64>() {
        return seconds;
    }
    let parts: Vec<_> = value.split(':').collect();
    let numbers: Vec<u64> = parts.iter().filter_map(|part| part.parse().ok()).collect();
    if numbers.len() != parts.len() {
        return 0;
    }
    match numbers.as_slice() {
        [hours, minutes, seconds] => hours * 3600 + minutes * 60 + seconds,
        [minutes, seconds] => minutes * 60 + seconds,
        [seconds] => *seconds,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
          <channel>
            <title>Sample Show</title>
            <description>A &lt;b&gt;sample&lt;/b&gt; show.</description>
            <itunes:author>Sample Host</itunes:author>
            <itunes:image href="https://example.com/art.jpg"/>
            <item>
              <title>Episode One</title>
              <guid>ep-1</guid>
              <enclosure url="https://example.com/ep1.mp3" type="audio/mpeg" length="1000"/>
              <pubDate>Wed, 14 Oct 2026 00:00:00 +0000</pubDate>
              <itunes:duration>1:02:03</itunes:duration>
            </item>
          </channel>
        </rss>"#;

    #[test]
    fn parses_title_enclosure_and_duration() {
        let show = parse_rss(SAMPLE, "https://example.com/feed.xml").unwrap();
        assert_eq!(show.title, "Sample Show");
        assert_eq!(show.author, "Sample Host");
        assert_eq!(show.description, "A sample show.");
        assert_eq!(show.episodes.len(), 1);
        assert_eq!(show.episodes[0].title, "Episode One");
        assert_eq!(show.episodes[0].audio_url, "https://example.com/ep1.mp3");
        assert_eq!(show.episodes[0].duration_secs, 3723);
        assert_eq!(show.episodes[0].guid, "ep-1");
        assert_eq!(show.episodes[0].published_secs, Some(1_791_936_000));
    }
}
