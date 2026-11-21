use async_trait::async_trait;
use rspotify::{
    clients::{BaseClient, OAuthClient},
    http::HttpError,
    model::{Id, LibraryId, PlayableItem, TrackId},
    AuthCodePkceSpotify, ClientError,
};
use serde_json::json;

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
    async fn playback(&self) -> Result<Option<Playback>, Error> {
        self.client
            .current_playback(None, None::<[&rspotify::model::AdditionalType; 0]>)
            .await
            .map(|context| {
                context.map(|context| Playback {
                    is_playing: context.is_playing,
                    volume_pct: context
                        .device
                        .volume_percent
                        .and_then(|value| u8::try_from(value).ok()),
                    position_ms: context
                        .progress
                        .and_then(|value| u64::try_from(value.num_milliseconds()).ok()),
                    track: context.item.as_ref().and_then(map_item),
                })
            })
            .map_err(map_error)
    }

    async fn queue(&self) -> Result<Vec<Track>, Error> {
        self.client
            .current_user_queue()
            .await
            .map(|queue| queue.queue.iter().filter_map(map_item).collect())
            .map_err(map_error)
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

fn map_item(item: &PlayableItem) -> Option<Track> {
    let PlayableItem::Track(track) = item else {
        return None;
    };
    Some(Track {
        id: track.id.as_ref().map(|id| id.id().to_owned()),
        title: track.name.clone(),
        artists: track
            .artists
            .iter()
            .map(|artist| artist.name.clone())
            .collect(),
        album: (!track.album.name.is_empty()).then(|| track.album.name.clone()),
        duration_ms: u64::try_from(track.duration.num_milliseconds()).unwrap_or(0),
        images: track
            .album
            .images
            .iter()
            .map(|image| Image {
                url: image.url.clone(),
                width: image.width,
                height: image.height,
            })
            .collect(),
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
        let item: PlayableItem = serde_json::from_str(TRACK_JSON).unwrap();
        let track = map_item(&item).unwrap();
        assert_eq!(track.title, "Song");
        assert_eq!(track.artists, ["Artist"]);
        assert_eq!(track.duration_ms, 123_000);
        assert_eq!(track.images.len(), 2);
    }
}
