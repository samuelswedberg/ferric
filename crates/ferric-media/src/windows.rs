use std::{
    cmp::Reverse,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use ferric_core::{
    command::Command,
    controller::{ControlError, Handled},
    model::{Art, Auth, Playback},
    store::Store,
};
use tokio::sync::mpsc;
use windows::{
    core::Result as WindowsResult,
    Foundation::TypedEventHandler,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession as WinSession,
        GlobalSystemMediaTransportControlsSessionManager as SessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as WinPlaybackStatus,
        MediaPropertiesChangedEventArgs, PlaybackInfoChangedEventArgs, SessionsChangedEventArgs,
        TimelinePropertiesChangedEventArgs,
    },
    Storage::Streams::DataReader,
};

use crate::{
    mapping::{
        map_playback, map_position, map_track, winrt_time_to_system_time, MediaMetadata,
        PlaybackStatus,
    },
    MediaError,
};

enum Event {
    SessionsChanged,
    SessionChanged,
}

struct Session {
    inner: WinSession,
    timeline_token: i64,
    playback_token: i64,
    media_token: i64,
}

impl Session {
    fn subscribe(inner: WinSession, tx: mpsc::UnboundedSender<Event>) -> Result<Self, MediaError> {
        let timeline_tx = tx.clone();
        let timeline_token = inner.TimelinePropertiesChanged(&TypedEventHandler::<
            WinSession,
            TimelinePropertiesChangedEventArgs,
        >::new(move |_, _| {
            let _ = timeline_tx.send(Event::SessionChanged);
            Ok(())
        }))?;

        let playback_tx = tx.clone();
        let playback_token = inner.PlaybackInfoChanged(&TypedEventHandler::<
            WinSession,
            PlaybackInfoChangedEventArgs,
        >::new(move |_, _| {
            let _ = playback_tx.send(Event::SessionChanged);
            Ok(())
        }))?;

        let media_token = inner.MediaPropertiesChanged(&TypedEventHandler::<
            WinSession,
            MediaPropertiesChangedEventArgs,
        >::new(move |_, _| {
            let _ = tx.send(Event::SessionChanged);
            Ok(())
        }))?;

        Ok(Self {
            inner,
            timeline_token,
            playback_token,
            media_token,
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self
            .inner
            .RemoveTimelinePropertiesChanged(self.timeline_token);
        let _ = self.inner.RemovePlaybackInfoChanged(self.playback_token);
        let _ = self.inner.RemoveMediaPropertiesChanged(self.media_token);
    }
}

pub(crate) async fn run(store: Store, art_cache_dir: PathBuf) -> Result<(), MediaError> {
    let manager = SessionManager::RequestAsync()?.await?;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let sessions_tx = tx.clone();
    let sessions_token = manager.SessionsChanged(&TypedEventHandler::<
        SessionManager,
        SessionsChangedEventArgs,
    >::new(move |_, _| {
        let _ = sessions_tx.send(Event::SessionsChanged);
        Ok(())
    }))?;

    // Errors inside the loop are logged and retried, never returned: a transient WinRT failure
    // (for example Spotify closing mid-read) must not stop media updates for the app's lifetime.
    let result: Result<(), MediaError> = async {
        let mut session: Option<Session> = None;
        let _ = tx.send(Event::SessionsChanged);

        loop {
            let event = if session.is_some() {
                rx.recv().await
            } else {
                tokio::select! {
                    event = rx.recv() => event,
                    () = tokio::time::sleep(Duration::from_secs(1)) => Some(Event::SessionsChanged),
                }
            };

            match event {
                Some(Event::SessionsChanged) => {
                    let found = match find_spotify(&manager) {
                        Ok(found) => found,
                        Err(error) => {
                            // Keep the current session; the next event or the 1 s retry tries again.
                            tracing::warn!(%error, "could not list media sessions");
                            continue;
                        }
                    };
                    session = match found
                        .map(|found| Session::subscribe(found, tx.clone()))
                        .transpose()
                    {
                        Ok(subscribed) => subscribed,
                        Err(error) => {
                            // With no session the loop re-checks every second.
                            tracing::warn!(%error, "could not subscribe to the Spotify session");
                            None
                        }
                    };
                    match &session {
                        Some(current) => {
                            if let Err(error) =
                                refresh(&store, &current.inner, &art_cache_dir).await
                            {
                                tracing::warn!(%error, "could not refresh Spotify media state");
                            }
                        }
                        None => mark_not_running(&store),
                    }
                }
                Some(Event::SessionChanged) => {
                    if let Some(current) = &session {
                        // A closing session can signal once before it disappears. Re-checking the
                        // manager keeps the last track while moving playback to Stopped.
                        match find_spotify(&manager) {
                            Ok(Some(_)) => {
                                if let Err(error) =
                                    refresh(&store, &current.inner, &art_cache_dir).await
                                {
                                    tracing::warn!(%error, "could not refresh Spotify media state");
                                }
                            }
                            Ok(None) => {
                                session = None;
                                mark_not_running(&store);
                            }
                            Err(error) => {
                                tracing::warn!(%error, "could not list media sessions");
                            }
                        }
                    }
                }
                None => return Ok(()),
            }
        }
    }
    .await;

    let _ = manager.RemoveSessionsChanged(sessions_token);
    result
}

pub(crate) async fn execute(cmd: &Command) -> Result<Handled, ControlError> {
    let operation = match cmd {
        Command::PlayPause => ControlOperation::Toggle,
        Command::Next => ControlOperation::Next,
        Command::Previous => ControlOperation::Previous,
        _ => return Ok(Handled::Unsupported),
    };

    let manager = SessionManager::RequestAsync()
        .map_err(control_error)?
        .await
        .map_err(control_error)?;
    let session = find_spotify(&manager)
        .map_err(control_error)?
        .ok_or(ControlError::NotRunning)?;
    let accepted = match operation {
        ControlOperation::Toggle => {
            session
                .TryTogglePlayPauseAsync()
                .map_err(control_error)?
                .await
        }
        ControlOperation::Next => session.TrySkipNextAsync().map_err(control_error)?.await,
        ControlOperation::Previous => session.TrySkipPreviousAsync().map_err(control_error)?.await,
    }
    .map_err(control_error)?;

    if accepted {
        Ok(Handled::Done)
    } else {
        Err(ControlError::Other(
            "Windows rejected the media command".into(),
        ))
    }
}

enum ControlOperation {
    Toggle,
    Next,
    Previous,
}

fn control_error(error: windows::core::Error) -> ControlError {
    ControlError::Other(error.to_string())
}

fn find_spotify(manager: &SessionManager) -> WindowsResult<Option<WinSession>> {
    let sessions = manager.GetSessions()?;
    for index in 0..sessions.Size()? {
        let session = sessions.GetAt(index)?;
        if session
            .SourceAppUserModelId()?
            .to_string_lossy()
            .to_lowercase()
            .contains("spotify")
        {
            return Ok(Some(session));
        }
    }
    Ok(None)
}

async fn refresh(store: &Store, session: &WinSession, cache_dir: &Path) -> Result<(), MediaError> {
    let properties = session.TryGetMediaPropertiesAsync()?.await?;
    let art = match properties.Thumbnail() {
        Ok(thumbnail) => cache_thumbnail(thumbnail, cache_dir).await?,
        Err(_) => None,
    };
    let metadata = MediaMetadata {
        title: properties.Title()?.to_string_lossy(),
        artist: properties.Artist()?.to_string_lossy(),
        album: properties.AlbumTitle()?.to_string_lossy(),
        art,
    };
    let playback_status = match session.GetPlaybackInfo()?.PlaybackStatus()? {
        WinPlaybackStatus::Playing => PlaybackStatus::Playing,
        WinPlaybackStatus::Paused => PlaybackStatus::Paused,
        _ => PlaybackStatus::Other,
    };
    // The SMTC position is valid as of LastUpdatedTime, which Spotify only moves on
    // play/pause/seek, so it must be the sample time for the UI's extrapolation.
    let timeline = session.GetTimelineProperties()?;
    let sampled_at = winrt_time_to_system_time(timeline.LastUpdatedTime()?.UniversalTime)
        .unwrap_or_else(SystemTime::now);
    let position = map_position(timeline.Position()?.Duration, sampled_at);

    store.update(|state| {
        state.spotify_running = true;
        state.track = Some(map_track(metadata, state.track.as_ref()));
        let playback = map_playback(playback_status);
        if !matches!(state.auth, Auth::SignedIn { .. }) {
            state.position = Some(position);
        } else if playback != state.playback {
            // Signed in, the position comes from the Spotify poll, which can be seconds
            // old. Re-anchor it at the pause/play edge so the display neither jumps back
            // on pause nor counts the paused time on resume.
            state.position = state.position.map(|current| {
                crate::mapping::rebase_position(
                    current,
                    state.playback,
                    state.track.as_ref().and_then(|track| track.duration_ms),
                    SystemTime::now(),
                )
            });
        }
        state.playback = playback;
    });
    Ok(())
}

fn mark_not_running(store: &Store) {
    store.update(|state| {
        state.spotify_running = false;
        state.playback = Playback::Stopped;
    });
}

async fn cache_thumbnail(
    reference: windows::Storage::Streams::IRandomAccessStreamReference,
    cache_dir: &Path,
) -> Result<Option<Art>, MediaError> {
    let stream = reference.OpenReadAsync()?.await?;
    let size = u32::try_from(stream.Size()?).map_err(|_| MediaError::ThumbnailTooLarge)?;
    if size == 0 {
        return Ok(None);
    }
    let input = stream.GetInputStreamAt(0)?;
    let reader = DataReader::CreateDataReader(&input)?;
    let loaded = reader.LoadAsync(size)?.await?;
    let mut bytes = vec![0; loaded as usize];
    reader.ReadBytes(&mut bytes)?;

    fs::create_dir_all(cache_dir)?;
    let cache_dir = cache_dir.canonicalize()?;
    let path = cache_dir.join(format!("{}.jpg", blake3::hash(&bytes).to_hex()));
    if !path.exists() {
        fs::write(&path, bytes)?;
    }
    prune_cache(&cache_dir)?;
    Ok(Some(Art::File {
        path: path.to_string_lossy().into_owned(),
    }))
}

fn prune_cache(cache_dir: &Path) -> Result<(), std::io::Error> {
    let mut files = fs::read_dir(cache_dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "jpg")
        })
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();
    files.sort_unstable_by_key(|(modified, _)| Reverse(*modified));
    for (_, path) in files.into_iter().skip(50) {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_pruning_keeps_fifty_most_recent_jpegs() {
        let cache = std::env::temp_dir().join(format!(
            "ferric-media-prune-{}",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&cache).unwrap();
        for index in 0..52 {
            fs::write(cache.join(format!("{index:02}.jpg")), []).unwrap();
        }
        prune_cache(&cache).unwrap();
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 50);
        fs::remove_dir_all(cache).unwrap();
    }
}
