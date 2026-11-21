use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub struct PlayerState {
    pub spotify_running: bool,
    pub track: Option<Track>,
    pub playback: Playback,
    pub position: Option<Position>,
    pub volume_pct: Option<u8>,
    pub liked: Option<bool>,
    pub queue: Option<Vec<Track>>,
    pub auth: Auth,
    pub theme: Option<Theme>,
    pub mode: Mode,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            spotify_running: false,
            track: None,
            playback: Playback::Stopped,
            position: None,
            volume_pct: None,
            liked: None,
            queue: None,
            auth: Auth::NoClientId,
            theme: None,
            mode: Mode::Full,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub struct Track {
    pub spotify_id: Option<String>,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    #[ts(type = "number | null")]
    pub duration_ms: Option<u64>,
    pub art: Option<Art>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub enum Art {
    File { path: String },
    Url { url: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub enum Playback {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub struct Position {
    #[ts(type = "number")]
    pub ms: u64,
    #[ts(type = "number")]
    pub sampled_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub enum Auth {
    NoClientId,
    SignedOut,
    SigningIn,
    SignedIn { display_name: String, premium: bool },
    Error { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub struct Theme {
    /// Cache key: the Spotify album id, or a hash of the art bytes.
    pub album_key: String,
    /// At least 4.5:1 contrast against the plinth (`#1E1B18`).
    pub accent: Rgb,
    /// Text and icons drawn on top of the accent colour.
    pub on_accent: Rgb,
    /// Worn sleeve: the cover with ring wear, rubbed corners and a crease (PNG).
    pub sleeve: Option<Art>,
    /// Dusty record with the worn cover as its label, transparent outside the disc (PNG).
    pub record: Option<Art>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../ui/src/lib/bindings/")]
pub enum Mode {
    Full,
    Card,
    Strip,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn round_trip<T>(value: &T, expected: Value)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let serialized = serde_json::to_value(value).expect("serialization should succeed");
        assert_eq!(serialized, expected);
        let deserialized: T =
            serde_json::from_value(serialized).expect("deserialization should succeed");
        assert_eq!(&deserialized, value);
    }

    #[test]
    fn default_player_state_matches_contract() {
        assert_eq!(
            PlayerState::default(),
            PlayerState {
                spotify_running: false,
                track: None,
                playback: Playback::Stopped,
                position: None,
                volume_pct: None,
                liked: None,
                queue: None,
                auth: Auth::NoClientId,
                theme: None,
                mode: Mode::Full,
            }
        );
    }

    #[test]
    fn art_uses_internally_tagged_camel_case_shape() {
        round_trip(
            &Art::File {
                path: "C:/art.jpg".into(),
            },
            json!({"kind": "file", "path": "C:/art.jpg"}),
        );
        round_trip(
            &Art::Url {
                url: "https://i.scdn.co/art.jpg".into(),
            },
            json!({"kind": "url", "url": "https://i.scdn.co/art.jpg"}),
        );
    }

    #[test]
    fn theme_carries_sleeve_and_record_images() {
        round_trip(
            &Theme {
                album_key: "abc123".into(),
                accent: Rgb {
                    r: 200,
                    g: 85,
                    b: 61,
                },
                on_accent: Rgb { r: 0, g: 0, b: 0 },
                sleeve: Some(Art::File {
                    path: "C:/cache/sleeve-abc123.png".into(),
                }),
                record: None,
            },
            json!({
                "albumKey": "abc123",
                "accent": {"r": 200, "g": 85, "b": 61},
                "onAccent": {"r": 0, "g": 0, "b": 0},
                "sleeve": {"kind": "file", "path": "C:/cache/sleeve-abc123.png"},
                "record": null
            }),
        );
    }

    #[test]
    fn playback_variants_are_camel_case() {
        round_trip(&Playback::Playing, json!("playing"));
        round_trip(&Playback::Paused, json!("paused"));
        round_trip(&Playback::Stopped, json!("stopped"));
    }

    #[test]
    fn auth_uses_internally_tagged_camel_case_shape() {
        round_trip(&Auth::NoClientId, json!({"status": "noClientId"}));
        round_trip(&Auth::SignedOut, json!({"status": "signedOut"}));
        round_trip(&Auth::SigningIn, json!({"status": "signingIn"}));
        round_trip(
            &Auth::SignedIn {
                display_name: "Ferric User".into(),
                premium: true,
            },
            json!({"status": "signedIn", "displayName": "Ferric User", "premium": true}),
        );
        round_trip(
            &Auth::Error {
                message: "denied".into(),
            },
            json!({"status": "error", "message": "denied"}),
        );
    }

    #[test]
    fn mode_variants_are_camel_case() {
        round_trip(&Mode::Full, json!("full"));
        round_trip(&Mode::Card, json!("card"));
        round_trip(&Mode::Strip, json!("strip"));
    }

    #[test]
    fn structs_use_camel_case_fields() {
        let track = Track {
            spotify_id: Some("track-id".into()),
            title: "Title".into(),
            artists: vec!["Artist".into()],
            album: Some("Album".into()),
            duration_ms: Some(123_000),
            art: None,
        };
        round_trip(
            &track,
            json!({
                "spotifyId": "track-id",
                "title": "Title",
                "artists": ["Artist"],
                "album": "Album",
                "durationMs": 123000,
                "art": null
            }),
        );

        round_trip(
            &Position {
                ms: 42,
                sampled_at_unix_ms: 1_700_000_000_000,
            },
            json!({"ms": 42, "sampledAtUnixMs": 1700000000000_u64}),
        );
    }
}
