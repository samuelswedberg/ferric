use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use ferric_core::{
    command::Command,
    controller::{ControlError, Controller, Handled},
    store::Store,
};

use crate::{
    auth::Authenticator,
    poller::{
        api::{Error, ErrorKind, RspotifyApi, SpotifyApi},
        notify_command,
    },
};

pub struct SpotifyController {
    store: Store,
    auth: Arc<Authenticator>,
    api_override: Option<Arc<dyn SpotifyApi>>,
}

impl SpotifyController {
    pub fn new(store: Store, auth: Arc<Authenticator>) -> Self {
        Self {
            store,
            auth,
            api_override: None,
        }
    }

    fn api(&self) -> Result<Arc<dyn SpotifyApi>, ControlError> {
        if let Some(api) = &self.api_override {
            return Ok(Arc::clone(api));
        }
        self.auth
            .client()
            .map(|client| Arc::new(RspotifyApi::new(client)) as Arc<dyn SpotifyApi>)
            .ok_or(ControlError::NotSignedIn)
    }

    async fn player_call<T, F, Fut>(
        &self,
        api: &dyn SpotifyApi,
        mut call: F,
    ) -> Result<T, ControlError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, Error>>,
    {
        match call().await {
            Err(error) if error.kind == ErrorKind::Unauthorized => {
                api.refresh()
                    .await
                    .map_err(|error| map_control_error(error, true))?;
                call().await.map_err(|error| map_control_error(error, true))
            }
            result => result.map_err(|error| map_control_error(error, true)),
        }
    }
}

#[async_trait]
impl Controller for SpotifyController {
    fn name(&self) -> &'static str {
        "spotify"
    }

    async fn execute(&self, cmd: &Command) -> Result<Handled, ControlError> {
        let api = self.api()?;
        match cmd {
            Command::PlayPause => {
                let playback = self.player_call(api.as_ref(), || api.playback()).await?;
                let playback = playback.ok_or(ControlError::NotRunning)?;
                if playback.is_playing {
                    self.player_call(api.as_ref(), || api.pause()).await?;
                } else {
                    self.player_call(api.as_ref(), || api.resume()).await?;
                }
            }
            Command::Next => self.player_call(api.as_ref(), || api.next()).await?,
            Command::Previous => self.player_call(api.as_ref(), || api.previous()).await?,
            Command::Seek { position_ms } => {
                self.player_call(api.as_ref(), || api.seek(*position_ms))
                    .await?;
                // Show the new spot now instead of waiting for the next poll.
                let sampled_at_unix_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
                    .try_into()
                    .unwrap_or(u64::MAX);
                self.store.update(|state| {
                    state.position = Some(ferric_core::model::Position {
                        ms: *position_ms,
                        sampled_at_unix_ms,
                    });
                });
            }
            Command::SetVolume { pct } => {
                if *pct > 100 {
                    return Err(ControlError::Other("volume must be 0..=100".into()));
                }
                self.auth.note_volume_set();
                self.player_call(api.as_ref(), || api.volume(*pct)).await?;
                self.auth.note_volume_set();
                self.store.update(|state| state.volume_pct = Some(*pct));
            }
            Command::SetLiked { liked } => {
                let track_id = self
                    .store
                    .get()
                    .track
                    .and_then(|track| track.spotify_id)
                    .ok_or_else(|| ControlError::Other("current track has no Spotify id".into()))?;
                api.set_liked(&track_id, *liked)
                    .await
                    .map_err(|error| map_control_error(error, false))?;
                self.store.update(|state| state.liked = Some(*liked));
            }
            Command::JumpTo { queue_index } => {
                if *queue_index > 9 {
                    return Err(ControlError::Other("too far".into()));
                }
                let count = queue_index + 1;
                for index in 0..count {
                    self.player_call(api.as_ref(), || api.next()).await?;
                    if index + 1 < count {
                        tokio::time::sleep(Duration::from_millis(150)).await;
                    }
                }
            }
        }
        notify_command();
        Ok(Handled::Done)
    }
}

fn map_control_error(error: Error, player_endpoint: bool) -> ControlError {
    match error.kind {
        ErrorKind::Unauthorized => ControlError::NotSignedIn,
        ErrorKind::Forbidden if player_endpoint => ControlError::PremiumRequired,
        ErrorKind::NotFound if player_endpoint => ControlError::NotRunning,
        ErrorKind::RateLimited { retry_after_secs } => ControlError::RateLimited(retry_after_secs),
        _ => ControlError::Other(error.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::MemoryTokenStore;
    use ferric_core::model::Track;
    use std::sync::Mutex;

    struct MockApi {
        calls: Mutex<Vec<String>>,
        playback: Option<crate::poller::api::Playback>,
    }

    impl MockApi {
        fn new(is_playing: bool) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                playback: Some(crate::poller::api::Playback {
                    is_playing,
                    volume_pct: Some(50),
                    position_ms: Some(0),
                    track: None,
                }),
            }
        }
        fn record(&self, value: impl Into<String>) {
            self.calls.lock().unwrap().push(value.into());
        }
    }

    #[async_trait]
    impl SpotifyApi for MockApi {
        async fn playback(&self) -> Result<Option<crate::poller::api::Playback>, Error> {
            self.record("playback");
            Ok(self.playback.clone())
        }
        async fn queue(&self) -> Result<Vec<crate::poller::api::Track>, Error> {
            Ok(Vec::new())
        }
        async fn library_contains(&self, _: &str) -> Result<bool, Error> {
            Ok(false)
        }
        async fn refresh(&self) -> Result<(), Error> {
            self.record("refresh");
            Ok(())
        }
        async fn pause(&self) -> Result<(), Error> {
            self.record("pause");
            Ok(())
        }
        async fn resume(&self) -> Result<(), Error> {
            self.record("resume");
            Ok(())
        }
        async fn next(&self) -> Result<(), Error> {
            self.record("next");
            Ok(())
        }
        async fn previous(&self) -> Result<(), Error> {
            self.record("previous");
            Ok(())
        }
        async fn seek(&self, position_ms: u64) -> Result<(), Error> {
            self.record(format!("seek:{position_ms}"));
            Ok(())
        }
        async fn volume(&self, pct: u8) -> Result<(), Error> {
            self.record(format!("volume:{pct}"));
            Ok(())
        }
        async fn set_liked(&self, _: &str, liked: bool) -> Result<(), Error> {
            self.record(format!("liked:{liked}"));
            Ok(())
        }
    }

    fn controller(api: Arc<MockApi>) -> SpotifyController {
        let store = Store::new();
        store.update(|state| {
            state.track = Some(Track {
                spotify_id: Some("track1".into()),
                title: "Song".into(),
                artists: vec!["Artist".into()],
                album: None,
                album_id: None,
                duration_ms: None,
                art: None,
            })
        });
        let auth = Arc::new(Authenticator::new(
            store.clone(),
            Arc::new(MemoryTokenStore::default()),
        ));
        SpotifyController {
            store,
            auth,
            api_override: Some(api),
        }
    }

    #[tokio::test]
    async fn play_pause_reads_state_then_pauses() {
        let api = Arc::new(MockApi::new(true));
        let controller = controller(Arc::clone(&api));
        assert_eq!(
            controller.execute(&Command::PlayPause).await,
            Ok(Handled::Done)
        );
        assert_eq!(*api.calls.lock().unwrap(), ["playback", "pause"]);
    }

    #[tokio::test]
    async fn optimistic_updates_follow_success() {
        let api = Arc::new(MockApi::new(false));
        let controller = controller(api);
        controller
            .execute(&Command::SetVolume { pct: 70 })
            .await
            .unwrap();
        controller
            .execute(&Command::SetLiked { liked: true })
            .await
            .unwrap();
        assert_eq!(controller.store.get().volume_pct, Some(70));
        assert_eq!(controller.store.get().liked, Some(true));
    }

    #[tokio::test]
    async fn remaining_commands_dispatch_to_the_expected_endpoints() {
        let api = Arc::new(MockApi::new(false));
        let controller = controller(Arc::clone(&api));
        controller.execute(&Command::PlayPause).await.unwrap();
        controller.execute(&Command::Next).await.unwrap();
        controller.execute(&Command::Previous).await.unwrap();
        controller
            .execute(&Command::Seek { position_ms: 4321 })
            .await
            .unwrap();
        assert_eq!(
            *api.calls.lock().unwrap(),
            ["playback", "resume", "next", "previous", "seek:4321"]
        );
    }

    #[tokio::test]
    async fn jump_rejects_far_index_and_advances_index_plus_one() {
        let api = Arc::new(MockApi::new(false));
        let controller = controller(Arc::clone(&api));
        assert_eq!(
            controller
                .execute(&Command::JumpTo { queue_index: 10 })
                .await,
            Err(ControlError::Other("too far".into()))
        );
        controller
            .execute(&Command::JumpTo { queue_index: 1 })
            .await
            .unwrap();
        assert_eq!(*api.calls.lock().unwrap(), ["next", "next"]);
    }

    #[test]
    fn player_http_failures_map_to_contract_errors() {
        let error = |kind| Error {
            kind,
            message: "spotify error".into(),
        };
        assert_eq!(
            map_control_error(error(ErrorKind::Forbidden), true),
            ControlError::PremiumRequired
        );
        assert_eq!(
            map_control_error(error(ErrorKind::NotFound), true),
            ControlError::NotRunning
        );
        assert_eq!(
            map_control_error(
                error(ErrorKind::RateLimited {
                    retry_after_secs: 9
                }),
                true
            ),
            ControlError::RateLimited(9)
        );
    }
}
