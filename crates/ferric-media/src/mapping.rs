use std::time::{SystemTime, UNIX_EPOCH};

use ferric_core::model::{Art, Playback, Position, Track};

pub(crate) struct MediaMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art: Option<Art>,
}

pub(crate) fn map_track(metadata: MediaMetadata, previous: Option<&Track>) -> Track {
    let artists = metadata
        .artist
        .split(", ")
        .filter(|artist| !artist.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let same_track = previous.is_some_and(|track| {
        track.title == metadata.title && track.artists.first() == artists.first()
    });

    Track {
        spotify_id: same_track
            .then(|| previous.and_then(|track| track.spotify_id.clone()))
            .flatten(),
        title: metadata.title,
        artists,
        artist_id: same_track
            .then(|| previous.and_then(|track| track.artist_id.clone()))
            .flatten(),
        album: (!metadata.album.is_empty()).then_some(metadata.album),
        album_id: same_track
            .then(|| previous.and_then(|track| track.album_id.clone()))
            .flatten(),
        duration_ms: same_track
            .then(|| previous.and_then(|track| track.duration_ms))
            .flatten(),
        // Once the Spotify poller has supplied the (larger) cover for this track, keep it:
        // swapping back to the media-session thumbnail on every update would flip the
        // theme between two covers and redraw the sleeve every couple of seconds.
        art: same_track
            .then(|| previous.and_then(|track| track.art.clone()))
            .flatten()
            .filter(|art| matches!(art, Art::Url { .. }))
            .or(metadata.art),
    }
}

pub(crate) fn map_playback(status: PlaybackStatus) -> Playback {
    match status {
        PlaybackStatus::Playing => Playback::Playing,
        PlaybackStatus::Paused => Playback::Paused,
        PlaybackStatus::Other => Playback::Stopped,
    }
}

#[derive(Clone, Copy)]
pub(crate) enum PlaybackStatus {
    Playing,
    Paused,
    Other,
}

/// 100 ns ticks between the WinRT epoch (1601-01-01) and the Unix epoch.
const WINRT_TO_UNIX_TICKS: i64 = 116_444_736_000_000_000;

/// Converts a WinRT `DateTime::UniversalTime` (100 ns ticks since 1601-01-01 UTC)
/// to a `SystemTime`. Returns `None` for unset (zero) or pre-1970 values.
pub(crate) fn winrt_time_to_system_time(universal_time: i64) -> Option<SystemTime> {
    let since_unix = u64::try_from(universal_time.checked_sub(WINRT_TO_UNIX_TICKS)?).ok()?;
    Some(UNIX_EPOCH + std::time::Duration::from_nanos(since_unix.saturating_mul(100)))
}

/// `ticks_100ns` is the SMTC position, which is valid as of `sampled_at`
/// (the timeline's `LastUpdatedTime`), not as of now.
pub(crate) fn map_position(ticks_100ns: i64, sampled_at: SystemTime) -> Position {
    let ms = u64::try_from(ticks_100ns).unwrap_or_default() / 10_000;
    let sampled_at_unix_ms = sampled_at
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    Position {
        ms,
        sampled_at_unix_ms,
    }
}

/// The position as of `now`, assuming it advanced in real time while `previous` was
/// Playing, and stamped `now` so later extrapolation starts from this moment.
pub(crate) fn rebase_position(
    current: Position,
    previous: Playback,
    duration_ms: Option<u64>,
    now: SystemTime,
) -> Position {
    let now_ms: u64 = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let mut ms = current.ms;
    if previous == Playback::Playing {
        ms = ms.saturating_add(now_ms.saturating_sub(current.sampled_at_unix_ms));
    }
    if let Some(duration) = duration_ms {
        ms = ms.min(duration);
    }
    Position {
        ms,
        sampled_at_unix_ms: now_ms,
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn rebase_advances_while_playing_and_freezes_while_paused() {
        let now = UNIX_EPOCH + Duration::from_millis(13_000);
        let current = Position {
            ms: 5_000,
            sampled_at_unix_ms: 10_000,
        };
        let paused_at = rebase_position(current, Playback::Playing, Some(60_000), now);
        assert_eq!(paused_at.ms, 8_000);
        assert_eq!(paused_at.sampled_at_unix_ms, 13_000);

        let resumed = rebase_position(current, Playback::Paused, None, now);
        assert_eq!(resumed.ms, 5_000);
        assert_eq!(resumed.sampled_at_unix_ms, 13_000);

        let clamped = rebase_position(current, Playback::Playing, Some(6_000), now);
        assert_eq!(clamped.ms, 6_000);
    }
    use super::*;
    use std::time::Duration;

    fn metadata(title: &str, artist: &str) -> MediaMetadata {
        MediaMetadata {
            title: title.into(),
            artist: artist.into(),
            album: "Album".into(),
            art: None,
        }
    }

    #[test]
    fn track_splits_artists_and_maps_empty_album() {
        let mut input = metadata("Song", "One, Two");
        input.album.clear();
        let track = map_track(input, None);
        assert_eq!(track.artists, ["One", "Two"]);
        assert_eq!(track.album, None);
    }

    #[test]
    fn track_preserves_poller_fields_for_same_title_and_first_artist() {
        let previous = Track {
            spotify_id: Some("spotify-id".into()),
            title: "Song".into(),
            artists: vec!["One".into()],
            artist_id: Some("artist-id".into()),
            album: None,
            album_id: Some("album-id".into()),
            duration_ms: Some(42),
            art: None,
        };
        let track = map_track(metadata("Song", "One, Guest"), Some(&previous));
        assert_eq!(track.spotify_id.as_deref(), Some("spotify-id"));
        assert_eq!(track.artist_id.as_deref(), Some("artist-id"));
        assert_eq!(track.album_id.as_deref(), Some("album-id"));
        assert_eq!(track.duration_ms, Some(42));
    }

    #[test]
    fn track_keeps_the_spotify_cover_for_the_same_track() {
        let previous = Track {
            spotify_id: Some("spotify-id".into()),
            title: "Song".into(),
            artists: vec!["One".into()],
            artist_id: None,
            album: None,
            album_id: None,
            duration_ms: None,
            art: Some(Art::Url {
                url: "https://i.scdn.co/640.jpg".into(),
            }),
        };
        let mut input = metadata("Song", "One");
        input.art = Some(Art::File {
            path: "thumb.jpg".into(),
        });
        let track = map_track(input, Some(&previous));
        assert_eq!(
            track.art,
            Some(Art::Url {
                url: "https://i.scdn.co/640.jpg".into()
            })
        );

        let mut next = metadata("Other", "One");
        next.art = Some(Art::File {
            path: "thumb2.jpg".into(),
        });
        let track = map_track(next, Some(&previous));
        assert_eq!(
            track.art,
            Some(Art::File {
                path: "thumb2.jpg".into()
            })
        );
    }

    #[test]
    fn track_drops_poller_fields_when_identity_changes() {
        let previous = Track {
            spotify_id: Some("spotify-id".into()),
            title: "Old".into(),
            artists: vec!["One".into()],
            artist_id: Some("artist-id".into()),
            album: None,
            album_id: Some("album-id".into()),
            duration_ms: Some(42),
            art: None,
        };
        let track = map_track(metadata("New", "One"), Some(&previous));
        assert_eq!(track.spotify_id, None);
        assert_eq!(track.artist_id, None);
        assert_eq!(track.album_id, None);
        assert_eq!(track.duration_ms, None);
    }

    #[test]
    fn playback_status_maps_to_contract() {
        assert_eq!(map_playback(PlaybackStatus::Playing), Playback::Playing);
        assert_eq!(map_playback(PlaybackStatus::Paused), Playback::Paused);
        assert_eq!(map_playback(PlaybackStatus::Other), Playback::Stopped);
    }

    #[test]
    fn position_converts_windows_ticks_and_samples_clock() {
        let now = UNIX_EPOCH + Duration::from_millis(1234);
        assert_eq!(
            map_position(9_876_540, now),
            Position {
                ms: 987,
                sampled_at_unix_ms: 1234,
            }
        );
    }

    #[test]
    fn winrt_time_converts_to_unix_time() {
        // 2026-01-01T00:00:00Z = 1_767_225_600 s after the Unix epoch.
        let ticks = 116_444_736_000_000_000 + 1_767_225_600 * 10_000_000;
        assert_eq!(
            winrt_time_to_system_time(ticks),
            Some(UNIX_EPOCH + Duration::from_secs(1_767_225_600))
        );
    }

    #[test]
    fn unset_or_pre_unix_winrt_time_is_none() {
        assert_eq!(winrt_time_to_system_time(0), None);
        assert_eq!(winrt_time_to_system_time(116_444_735_999_999_999), None);
    }

    #[test]
    fn negative_position_is_safely_clamped() {
        assert_eq!(map_position(-1, UNIX_EPOCH).ms, 0);
    }
}
