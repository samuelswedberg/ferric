# T01: Media watcher (Windows SMTC)

| Meta | |
| --- | --- |
| Wave | 1 |
| Depends on | T00 |
| Branch | `feat/t01-media-watcher` |
| Milestone | M0 |
| PR title | `feat(media): watch Spotify via Windows media controls` |

## Goal

Keep `PlayerState` in sync with the Spotify desktop app through Windows' Global System Media Transport Controls (SMTC), with no login. Also implement `MediaController`, so play/pause/next/previous work for everyone.

## Context

`docs/CONTRACTS.md` §1 (the fields this task writes are in the "Who writes which field" table), §3 and §5. Use the `windows` crate, features `Media_Control`, `Foundation`, `Foundation_Collections`, `Storage_Streams`.

## Owns

`crates/ferric-media/**`

## Spec

- `run(store, art_cache_dir)`:
  - `GlobalSystemMediaTransportControlsSessionManager::RequestAsync()`.
  - Pick the session whose `SourceAppUserModelId` contains `Spotify` (case-insensitive). Ignore every other app's session, even if Windows marks it current.
  - Subscribe to `SessionsChanged`, `MediaPropertiesChanged`, `PlaybackInfoChanged` and `TimelinePropertiesChanged`, and re-read on each event. No polling loop, except a 1 s re-check while no Spotify session exists.
  - Write `spotify_running`, `track` (title, artists split on `", "`, album, art), `playback`, `position` (only while `auth` is not `SignedIn`).
  - Thumbnail: read the stream into bytes, write it to `art_cache_dir/<blake3 or sha1 of bytes>.jpg` (skip if it already exists), and set `Art::File`. Keep the 50 most recent files and delete the rest.
  - When Spotify closes: `spotify_running = false`, `playback = Stopped`, and keep the last track.
  - Don't overwrite `track.spotify_id` / `duration_ms` (owned by the poller) when the title and artist haven't changed.
- `MediaController::execute`: `PlayPause` → `TryTogglePlayPauseAsync`, `Next` → `TrySkipNextAsync`, `Previous` → `TrySkipPreviousAsync`. Return `NotRunning` when there's no Spotify session. Anything else → `Ok(Handled::Unsupported)`.
- On non-Windows targets: `run` returns `Ok(())` straight away, and the controller returns `Unsupported`.

## Steps

1. Write a `Session` wrapper that turns WinRT async calls into futures (`.await` on the `IAsyncOperation`, or `spawn_blocking` + `.get()`).
2. Write the mapping functions (WinRT props → `Track`/`Playback`/`Position`) as pure functions, with unit tests.
3. Hook up the event handlers, pushing changes into the store through an mpsc channel (don't call `store.update` from WinRT callback threads with locks held).
4. Thumbnail caching.
5. Add a small example, `cargo run -p ferric-media --example watch`, that prints each state change.

## Acceptance criteria

- [ ] With Spotify playing, the example prints the correct title/artist/album, and a new line within 200 ms of skipping a track
- [ ] Pause and resume in Spotify are reflected; closing Spotify sets `spotify_running=false`
- [ ] Playing a YouTube video in a browser at the same time does **not** change the reported track
- [ ] The thumbnail file exists and opens as an image; repeated tracks reuse the same file
- [ ] The example's `--toggle`, `--next` and `--prev` flags control Spotify through `MediaController`
- [ ] Unit tests cover the mapping functions; `cargo clippy -p ferric-media -- -D warnings` is clean
- [ ] `cargo check` passes for a non-Windows target (`--target x86_64-unknown-linux-gnu`, if that target is installed; otherwise review the `cfg` code by reading it)

## Out of scope

Web API anything, themes, wiring into the Tauri app (T08).
