use async_trait::async_trait;
use rspotify::{
    clients::{BaseClient, OAuthClient},
    http::HttpError,
    model::{LibraryId, TrackId},
    AuthCodePkceSpotify, ClientError,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Image {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Track {
    pub id: Option<String>,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub duration_ms: u64,
    pub images: Vec<Image>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Playback {
    pub is_playing: bool,
    pub volume_pct: Option<u8>,
    pub position_ms: Option<u64>,
    pub track: Option<Track>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited { retry_after_secs: u64 },
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

#[async_trait]
pub(crate) trait SpotifyApi: Send + Sync {
    async fn playback(&self) -> Result<Option<Playback>, Error>;
    async fn queue(&self) -> Result<Vec<Track>, Error>;
    async fn library_contains(&self, track_id: &str) -> Result<bool, Error>;
    async fn refresh(&self) -> Result<(), Error>;
    async fn pause(&self) -> Result<(), Error>;
    async fn resume(&self) -> Result<(), Error>;
    async fn next(&self) -> Result<(), Error>;
    async fn previous(&self) -> Result<(), Error>;
    async fn seek(&self, position_ms: u64) -> Result<(), Error>;
    async fn volume(&self, pct: u8) -> Result<(), Error>;
    async fn set_liked(&self, track_id: &str, liked: bool) -> Result<(), Error>;
}

#[derive(Clone)]
pub(crate) struct RspotifyApi {
    client: AuthCodePkceSpotify,
}

impl RspotifyApi {
    pub(crate) fn new(client: AuthCodePkceSpotify) -> Self {
        Self { client }
    }

    async fn empty_put(&self, path: &str) -> Result<(), Error> {
        self.client
            .api_put(path, &json!({}))
            .await
            .map(|_| ())
            .map_err(map_error)
    }

    async fn empty_post(&self, path: &str) -> Result<(), Error> {
        self.client
            .api_post(path, &json!({}))
            .await
            .map(|_| ())
            .map_err(map_error)
    }
}

#[async_trait]
impl SpotifyApi for RspotifyApi {
    // Playback and queue are fetched raw and parsed with Ferric's own lenient types:
    // rspotify's typed models require track fields Spotify no longer returns to
    // development-mode apps, and then silently drop the whole track.
    async fn playback(&self) -> Result<Option<Playback>, Error> {
        let body = self
            .client
            .api_get("me/player", &Default::default())
            .await
            .map_err(map_error)?;
        Ok(parse_playback(&body))
    }

    async fn queue(&self) -> Result<Vec<Track>, Error> {
        let body = self
            .client
            .api_get("me/player/queue", &Default::default())
            .await
            .map_err(map_error)?;
        Ok(parse_queue(&body))
    }

    async fn library_contains(&self, track_id: &str) -> Result<bool, Error> {
        let id = TrackId::from_id(track_id).map_err(|error| Error {
            kind: ErrorKind::Other,
            message: error.to_string(),
        })?;
        self.client
            .library_contains([LibraryId::Track(id)])
            .await
            .map(|values| values.first().copied().unwrap_or(false))
            .map_err(map_error)
    }

    async fn refresh(&self) -> Result<(), Error> {
        self.client.refresh_token().await.map_err(map_error)
    }

    async fn pause(&self) -> Result<(), Error> {
        self.empty_put("me/player/pause").await
    }

    async fn resume(&self) -> Result<(), Error> {
        self.empty_put("me/player/play").await
    }

    async fn next(&self) -> Result<(), Error> {
        self.empty_post("me/player/next").await
    }

    async fn previous(&self) -> Result<(), Error> {
        self.empty_post("me/player/previous").await
    }

    async fn seek(&self, position_ms: u64) -> Result<(), Error> {
        self.empty_put(&format!("me/player/seek?position_ms={position_ms}"))
            .await
    }

    async fn volume(&self, pct: u8) -> Result<(), Error> {
        self.empty_put(&format!("me/player/volume?volume_percent={pct}"))
            .await
    }

    async fn set_liked(&self, track_id: &str, liked: bool) -> Result<(), Error> {
        let id = TrackId::from_id(track_id).map_err(|error| Error {
            kind: ErrorKind::Other,
            message: error.to_string(),
        })?;
        let item = LibraryId::Track(id);
        let result = if liked {
            self.client.library_add([item]).await
        } else {
            self.client.library_remove([item]).await
        };
        result.map_err(map_error)
    }
}

#[derive(Deserialize)]
struct RawPlayback {
    #[serde(default)]
    is_playing: bool,
    progress_ms: Option<u64>,
    device: Option<RawDevice>,
    item: Option<Value>,
}

#[derive(Deserialize)]
struct RawDevice {
    volume_percent: Option<u32>,
}

#[derive(Deserialize)]
struct RawQueue {
    #[serde(default)]
    queue: Vec<Value>,
}

#[derive(Deserialize)]
struct RawTrack {
    id: Option<String>,
    name: String,
    #[serde(default)]
    artists: Vec<RawNamed>,
    album: Option<RawAlbum>,
    #[serde(default)]
    duration_ms: u64,
}

#[derive(Deserialize)]
struct RawNamed {
    name: String,
}

#[derive(Deserialize)]
struct RawAlbum {
    id: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    images: Vec<RawImage>,
}

#[derive(Deserialize)]
struct RawImage {
    url: String,
    width: Option<u32>,
    height: Option<u32>,
}

/// `GET me/player`; an empty body (204) means nothing is playing.
fn parse_playback(body: &str) -> Option<Playback> {
    if body.trim().is_empty() {
        return None;
    }
    let raw: RawPlayback = match serde_json::from_str(body) {
        Ok(raw) => raw,
        Err(error) => {
            tracing::warn!(%error, "couldn't parse Spotify playback");
            return None;
        }
    };
    Some(Playback {
        is_playing: raw.is_playing,
        volume_pct: raw
            .device
            .and_then(|device| device.volume_percent)
            .and_then(|value| u8::try_from(value).ok()),
        position_ms: raw.progress_ms,
        track: raw.item.as_ref().and_then(parse_item),
    })
}

fn parse_queue(body: &str) -> Vec<Track> {
    match serde_json::from_str::<RawQueue>(body) {
        Ok(raw) => raw.queue.iter().filter_map(parse_item).collect(),
        Err(error) => {
            tracing::warn!(%error, "couldn't parse Spotify queue");
            Vec::new()
        }
    }
}

/// Maps a playable item; episodes and unreadable items are skipped.
fn parse_item(item: &Value) -> Option<Track> {
    let kind = item.get("type").and_then(Value::as_str);
    if kind.is_some_and(|kind| kind != "track") {
        return None;
    }
    let track = match RawTrack::deserialize(item) {
        Ok(track) => track,
        Err(error) => {
            tracing::debug!(%error, "skipping a Spotify item that isn't a readable track");
            return None;
        }
    };
    let album = track.album;
    Some(Track {
        id: track.id,
        title: track.name,
        artists: track
            .artists
            .into_iter()
            .map(|artist| artist.name)
            .collect(),
        album: album
            .as_ref()
            .map(|album| album.name.clone())
            .filter(|name| !name.is_empty()),
        album_id: album.as_ref().and_then(|album| album.id.clone()),
        duration_ms: track.duration_ms,
        images: album
            .map(|album| {
                album
                    .images
                    .into_iter()
                    .map(|image| Image {
                        url: image.url,
                        width: image.width,
                        height: image.height,
                    })
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn map_error(error: ClientError) -> Error {
    let message = error.to_string();
    let kind = match error {
        ClientError::Http(error) => match *error {
            HttpError::StatusCode(response) => match response.status().as_u16() {
                401 => ErrorKind::Unauthorized,
                403 => ErrorKind::Forbidden,
                404 => ErrorKind::NotFound,
                429 => {
                    let retry_after_secs = response
                        .headers()
                        .get("retry-after")
                        .and_then(|value| value.to_str().ok())
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(5);
                    ErrorKind::RateLimited { retry_after_secs }
                }
                _ => ErrorKind::Other,
            },
            HttpError::Client(_) => ErrorKind::Other,
        },
        ClientError::InvalidToken => ErrorKind::Unauthorized,
        _ => ErrorKind::Other,
    };
    Error { kind, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK_JSON: &str = r#"{
      "album":{"album_type":"album","artists":[],"available_markets":[],"external_urls":{},"href":null,"id":"album1","images":[{"height":640,"url":"https://i.scdn.co/640.jpg","width":640},{"height":300,"url":"https://i.scdn.co/300.jpg","width":300}],"name":"Album","release_date":null,"release_date_precision":null,"restrictions":null},
      "artists":[{"external_urls":{},"href":null,"id":"artist1","name":"Artist","type":"artist","uri":"spotify:artist:artist1"}],
      "available_markets":[],"disc_number":1,"duration_ms":123000,"explicit":false,"external_ids":{},"external_urls":{},"href":null,"id":"track1","is_local":false,"is_playable":true,"linked_from":null,"restrictions":null,"name":"Song","popularity":0,"preview_url":null,"track_number":1,"type":"track","uri":"spotify:track:track1"
    }"#;

    #[test]
    fn recorded_track_json_maps_to_internal_shape() {
        let item: Value = serde_json::from_str(TRACK_JSON).unwrap();
        let track = parse_item(&item).unwrap();
        assert_eq!(track.id.as_deref(), Some("track1"));
        assert_eq!(track.title, "Song");
        assert_eq!(track.artists, ["Artist"]);
        assert_eq!(track.album.as_deref(), Some("Album"));
        assert_eq!(track.album_id.as_deref(), Some("album1"));
        assert_eq!(track.duration_ms, 123_000);
        assert_eq!(track.images.len(), 2);
    }

    /// Spotify trims track objects for development-mode apps (no available_markets,
    /// external_ids, popularity, linked_from, ...). These must still parse.
    #[test]
    fn trimmed_track_json_still_parses() {
        let body = r#"{"is_playing":true,"progress_ms":4200,"device":{"volume_percent":42},
          "item":{"type":"track","id":"t2","name":"S.E.X. Narcissist","duration_ms":201000,
            "artists":[{"name":"VIOLENT VIRA"}],
            "album":{"id":"a2","name":"Lover Of A Ghost","images":[{"url":"https://i.scdn.co/x.jpg","width":640,"height":640}]}}}"#;
        let playback = parse_playback(body).unwrap();
        assert!(playback.is_playing);
        assert_eq!(playback.volume_pct, Some(42));
        assert_eq!(playback.position_ms, Some(4200));
        let track = playback.track.unwrap();
        assert_eq!(track.id.as_deref(), Some("t2"));
        assert_eq!(track.duration_ms, 201_000);
        assert_eq!(track.album_id.as_deref(), Some("a2"));
    }

    #[test]
    fn empty_playback_body_means_nothing_playing() {
        assert_eq!(parse_playback(""), None);
    }

    #[test]
    fn queue_skips_episodes_and_keeps_tracks() {
        let body = r#"{"currently_playing":null,"queue":[
          {"type":"episode","id":"e1","name":"Podcast","duration_ms":1},
          {"type":"track","id":"t3","name":"Next","duration_ms":1000,"artists":[{"name":"A"}],"album":{"id":"a3","name":"B","images":[]}}]}"#;
        let queue = parse_queue(body);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].title, "Next");
    }
}
