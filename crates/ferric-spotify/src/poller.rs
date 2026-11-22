use std::{
    collections::VecDeque,
    sync::{Arc, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use ferric_core::{
    model::{Art, Auth, Position, Track},
    store::Store,
};
use tokio::sync::watch;

use crate::auth::Authenticator;

#[path = "api.rs"]
pub(crate) mod api;

use api::{Error, ErrorKind, RspotifyApi, SpotifyApi};

const POLL_INTERVAL: Duration = Duration::from_secs(15);
const ENDPOINT_DEBOUNCE: Duration = Duration::from_secs(2);
const REQUEST_BUDGET: usize = 12;

static COMMAND_REVISION: OnceLock<watch::Sender<u64>> = OnceLock::new();

fn command_sender() -> &'static watch::Sender<u64> {
    COMMAND_REVISION.get_or_init(|| watch::channel(0).0)
}

pub(crate) fn notify_command() {
    let sender = command_sender();
    let next = sender.borrow().wrapping_add(1);
    sender.send_replace(next);
}

pub async fn run(store: Store, auth: Arc<Authenticator>, mut visible: watch::Receiver<bool>) {
    let mut state_rx = store.subscribe();
    let mut commands = command_sender().subscribe();
    let mut tick = tokio::time::interval(POLL_INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut poll_state = PollState::default();
    let mut last_identity = track_identity(&store.get().track);

    loop {
        enum Trigger {
            State,
            Visible,
            Tick,
            Command,
        }
        let trigger = tokio::select! {
            changed = state_rx.changed() => {
                if changed.is_err() { return; }
                Trigger::State
            }
            changed = visible.changed() => {
                if changed.is_err() { return; }
                Trigger::Visible
            }
            _ = tick.tick() => Trigger::Tick,
            changed = commands.changed() => {
                if changed.is_err() { return; }
                Trigger::Command
            }
        };

        let is_visible = *visible.borrow();
        let snapshot = store.get();
        let identity = track_identity(&snapshot.track);
        let track_changed = identity != last_identity;
        if track_changed {
            last_identity = identity;
        }
        let should_poll = match trigger {
            Trigger::State => track_changed || !matches!(snapshot.auth, Auth::SignedIn { .. }),
            Trigger::Visible | Trigger::Tick => is_visible,
            Trigger::Command => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                is_visible
            }
        };
        if !should_poll {
            continue;
        }

        let client = auth.client().map(RspotifyApi::new);
        let signed_in = matches!(snapshot.auth, Auth::SignedIn { .. });
        let api = client.as_ref().map(|api| api as &dyn SpotifyApi);
        if let Some(delay) = poll_cycle(&store, api, signed_in, is_visible, &mut poll_state).await {
            tokio::time::sleep(delay).await;
        }
    }
}

#[derive(Default)]
struct PollState {
    last_player: Option<Instant>,
    last_queue: Option<Instant>,
    last_liked: Option<Instant>,
    liked_track_id: Option<String>,
    requests: VecDeque<Instant>,
    backoff: Backoff,
}

impl PollState {
    fn ready(last: Option<Instant>, now: Instant) -> bool {
        last.is_none_or(|last| now.duration_since(last) >= ENDPOINT_DEBOUNCE)
    }

    fn note_request(&mut self, now: Instant) {
        while self
            .requests
            .front()
            .is_some_and(|request| now.duration_since(*request) >= Duration::from_secs(60))
        {
            self.requests.pop_front();
        }
        self.requests.push_back(now);
        if self.requests.len() > REQUEST_BUDGET {
            tracing::warn!(
                requests_last_minute = self.requests.len(),
                "Spotify polling exceeded the steady-state request budget"
            );
        }
    }
}

#[derive(Debug, Default)]
struct Backoff {
    interval: Option<Duration>,
}

impl Backoff {
    fn rate_limited(&mut self, retry_after_secs: u64) -> Duration {
        let retry_after = Duration::from_secs(retry_after_secs.max(1));
        let current = self.interval.unwrap_or(retry_after).max(retry_after);
        self.interval = Some((current * 2).min(Duration::from_secs(60)));
        current
    }

    fn success(&mut self) {
        self.interval = None;
    }
}

async fn poll_cycle(
    store: &Store,
    api: Option<&dyn SpotifyApi>,
    signed_in: bool,
    visible: bool,
    state: &mut PollState,
) -> Option<Duration> {
    if !signed_in || api.is_none() {
        store.update(|player| {
            player.queue = None;
            player.liked = None;
            player.volume_pct = None;
        });
        return None;
    }
    if !visible {
        return None;
    }
    let api = api.expect("checked above");
    let now = Instant::now();
    let mut matched_track_id = None;

    if PollState::ready(state.last_player, now) {
        state.last_player = Some(now);
        state.note_request(now);
        match with_refresh(api, || api.playback()).await {
            Ok(playback) => {
                state.backoff.success();
                matched_track_id = apply_playback(store, playback);
            }
            Err(error) => {
                if let Some(delay) = rate_limit_delay(state, &error) {
                    return Some(delay);
                }
                tracing::warn!(error = %error, "Spotify playback poll failed");
            }
        }
    }

    if let Some(track_id) = matched_track_id {
        let changed = state.liked_track_id.as_deref() != Some(track_id.as_str());
        if changed && PollState::ready(state.last_liked, now) {
            state.last_liked = Some(now);
            state.note_request(now);
            match with_refresh(api, || api.library_contains(&track_id)).await {
                Ok(liked) => {
                    state.backoff.success();
                    state.liked_track_id = Some(track_id);
                    store.update(|player| player.liked = Some(liked));
                }
                Err(error) => {
                    if let Some(delay) = rate_limit_delay(state, &error) {
                        return Some(delay);
                    }
                    tracing::warn!(error = %error, "Spotify library poll failed");
                }
            }
        }
    }

    if PollState::ready(state.last_queue, now) {
        state.last_queue = Some(now);
        state.note_request(now);
        match with_refresh(api, || api.queue()).await {
            Ok(queue) => {
                state.backoff.success();
                store.update(|player| {
                    player.queue = Some(queue.into_iter().map(map_track).collect())
                });
            }
            Err(error) => {
                if let Some(delay) = rate_limit_delay(state, &error) {
                    return Some(delay);
                }
                tracing::warn!(error = %error, "Spotify queue poll failed");
            }
        }
    }
    None
}

async fn with_refresh<T, F, Fut>(api: &dyn SpotifyApi, mut request: F) -> Result<T, Error>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
{
    match request().await {
        Err(error) if error.kind == ErrorKind::Unauthorized => {
            api.refresh().await?;
            request().await
        }
        result => result,
    }
}

fn rate_limit_delay(state: &mut PollState, error: &Error) -> Option<Duration> {
    match error.kind {
        ErrorKind::RateLimited { retry_after_secs } => {
            Some(state.backoff.rate_limited(retry_after_secs))
        }
        _ => None,
    }
}

fn apply_playback(store: &Store, playback: Option<api::Playback>) -> Option<String> {
    let Some(playback) = playback else {
        store.update(|player| {
            player.volume_pct = None;
            player.position = None;
        });
        return None;
    };
    let sampled_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let mut matched_id = None;
    store.update(|player| {
        player.volume_pct = playback.volume_pct;
        player.position = playback.position_ms.map(|ms| Position {
            ms,
            sampled_at_unix_ms,
        });
        if let (Some(current), Some(remote)) = (player.track.as_mut(), playback.track.as_ref()) {
            if same_track(current, remote) {
                matched_id = remote.id.clone();
                enrich_track(current, remote);
            }
        }
    });
    matched_id
}

fn same_track(current: &Track, remote: &api::Track) -> bool {
    normalized(&current.title) == normalized(&remote.title)
        && current
            .artists
            .first()
            .zip(remote.artists.first())
            .is_some_and(|(left, right)| normalized(left) == normalized(right))
}

fn normalized(value: &str) -> String {
    value.trim().to_lowercase()
}

fn enrich_track(current: &mut Track, remote: &api::Track) {
    current.spotify_id.clone_from(&remote.id);
    current.duration_ms = Some(remote.duration_ms);
    if let Some(image) = preferred_image(&remote.images) {
        current.art = Some(Art::Url {
            url: image.url.clone(),
        });
    }
}

fn preferred_image(images: &[api::Image]) -> Option<&api::Image> {
    images
        .iter()
        .filter(|image| image.width.unwrap_or(0) <= 640 && image.height.unwrap_or(0) <= 640)
        .max_by_key(|image| image.width.unwrap_or(0) * image.height.unwrap_or(0))
}

fn map_track(track: api::Track) -> Track {
    let art = preferred_image(&track.images).map(|image| Art::Url {
        url: image.url.clone(),
    });
    Track {
        spotify_id: track.id,
        title: track.title,
        artists: track.artists,
        album: track.album,
        album_id: track.album_id,
        duration_ms: Some(track.duration_ms),
        art,
    }
}

fn track_identity(track: &Option<Track>) -> Option<(String, String)> {
    track.as_ref().map(|track| {
        (
            normalized(&track.title),
            track
                .artists
                .first()
                .map_or_else(String::new, |artist| normalized(artist)),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ferric_core::model::Playback as PlaybackState;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingApi {
        calls: AtomicUsize,
        playback: Option<api::Playback>,
        queue: Vec<api::Track>,
        liked: bool,
    }

    impl Default for CountingApi {
        fn default() -> Self {
            Self {
                calls: AtomicUsize::new(0),
                playback: None,
                queue: Vec::new(),
                liked: false,
            }
        }
    }
    impl CountingApi {
        fn count(&self) -> usize {
            self.calls.load(Ordering::Relaxed)
        }
        fn called(&self) {
            self.calls.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[async_trait]
    impl SpotifyApi for CountingApi {
        async fn playback(&self) -> Result<Option<api::Playback>, Error> {
            self.called();
            Ok(self.playback.clone())
        }
        async fn queue(&self) -> Result<Vec<api::Track>, Error> {
            self.called();
            Ok(self.queue.clone())
        }
        async fn library_contains(&self, _: &str) -> Result<bool, Error> {
            self.called();
            Ok(self.liked)
        }
        async fn refresh(&self) -> Result<(), Error> {
            self.called();
            Ok(())
        }
        async fn pause(&self) -> Result<(), Error> {
            self.called();
            Ok(())
        }
        async fn resume(&self) -> Result<(), Error> {
            self.called();
            Ok(())
        }
        async fn next(&self) -> Result<(), Error> {
            self.called();
            Ok(())
        }
        async fn previous(&self) -> Result<(), Error> {
            self.called();
            Ok(())
        }
        async fn seek(&self, _: u64) -> Result<(), Error> {
            self.called();
            Ok(())
        }
        async fn volume(&self, _: u8) -> Result<(), Error> {
            self.called();
            Ok(())
        }
        async fn set_liked(&self, _: &str, _: bool) -> Result<(), Error> {
            self.called();
            Ok(())
        }
    }

    fn local_track(title: &str) -> Track {
        Track {
            spotify_id: None,
            title: title.into(),
            artists: vec!["Artist".into()],
            album: None,
            album_id: None,
            duration_ms: None,
            art: None,
        }
    }
    fn remote_track(title: &str) -> api::Track {
        api::Track {
            id: Some("track1".into()),
            title: title.into(),
            artists: vec![" artist ".into()],
            album: Some("Album".into()),
            album_id: Some("album1".into()),
            duration_ms: 1000,
            images: vec![api::Image {
                url: "https://i.scdn.co/640.jpg".into(),
                width: Some(640),
                height: Some(640),
            }],
        }
    }

    #[tokio::test]
    async fn hidden_or_signed_out_makes_no_request() {
        let store = Store::new();
        let api = CountingApi::default();
        let mut state = PollState::default();
        poll_cycle(&store, Some(&api), true, false, &mut state).await;
        poll_cycle(&store, Some(&api), false, true, &mut state).await;
        assert_eq!(api.count(), 0);
        assert_eq!(store.get().queue, None);
    }

    #[tokio::test]
    async fn visible_poll_enriches_track_queue_liked_volume_and_position() {
        let store = Store::new();
        store.update(|state| state.track = Some(local_track("Song")));
        let remote = remote_track("Song");
        let api = CountingApi {
            calls: AtomicUsize::new(0),
            playback: Some(api::Playback {
                is_playing: true,
                volume_pct: Some(65),
                position_ms: Some(750),
                track: Some(remote.clone()),
            }),
            queue: vec![remote],
            liked: true,
        };
        let mut state = PollState::default();
        poll_cycle(&store, Some(&api), true, true, &mut state).await;

        let snapshot = store.get();
        assert_eq!(api.count(), 3);
        assert_eq!(snapshot.volume_pct, Some(65));
        assert_eq!(snapshot.position.unwrap().ms, 750);
        assert_eq!(snapshot.liked, Some(true));
        assert_eq!(snapshot.queue.unwrap().len(), 1);
        assert_eq!(
            snapshot.track.unwrap().spotify_id.as_deref(),
            Some("track1")
        );
    }

    #[test]
    fn enrichment_requires_matching_title_and_first_artist() {
        let mut current = local_track("Different");
        let original = current.clone();
        let remote = remote_track("Song");
        assert!(!same_track(&current, &remote));
        if same_track(&current, &remote) {
            enrich_track(&mut current, &remote);
        }
        assert_eq!(current, original);
        current.title = " song ".into();
        assert!(same_track(&current, &remote));
        enrich_track(&mut current, &remote);
        assert_eq!(current.spotify_id.as_deref(), Some("track1"));
        assert_eq!(current.duration_ms, Some(1000));
    }

    #[test]
    fn rate_limit_backoff_doubles_and_caps_at_sixty_seconds() {
        let mut backoff = Backoff::default();
        assert_eq!(backoff.rate_limited(5), Duration::from_secs(5));
        assert_eq!(backoff.rate_limited(5), Duration::from_secs(10));
        assert_eq!(backoff.rate_limited(5), Duration::from_secs(20));
        assert_eq!(backoff.rate_limited(5), Duration::from_secs(40));
        assert_eq!(backoff.rate_limited(5), Duration::from_secs(60));
        backoff.success();
        assert_eq!(backoff.rate_limited(7), Duration::from_secs(7));
    }

    #[test]
    fn playback_mapping_keeps_core_playback_writer_untouched() {
        let store = Store::new();
        store.update(|state| {
            state.playback = PlaybackState::Playing;
            state.track = Some(local_track("Song"));
        });
        apply_playback(
            &store,
            Some(api::Playback {
                is_playing: false,
                volume_pct: Some(55),
                position_ms: Some(250),
                track: Some(remote_track("Song")),
            }),
        );
        assert_eq!(store.get().playback, PlaybackState::Playing);
        assert_eq!(store.get().volume_pct, Some(55));
    }
}
