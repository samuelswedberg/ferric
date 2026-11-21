# T08: App wiring (media + theme → UI)

| Meta | |
| --- | --- |
| Wave | 2 |
| Depends on | T00, T01, T03, T05 |
| Branch | `feat/t08-app-wiring` |
| Milestone | M0 (media part) / M1 (theme part) |
| PR title | `feat(app): wire media watcher, theme and command router` |

## Goal

Connect the pieces into a working app: the media watcher feeds the store, the theme follows the album art, every store change reaches the UI, and UI commands reach the right controller. After this PR, `cargo tauri dev` is a usable no-login miniplayer.

## Context

`docs/CONTRACTS.md` §3 (router order), §5, §6 (event + commands owned by T08). T04's UI may or may not be merged. Test with whatever `App.svelte` is on `main`.

## Owns

`src-tauri/src/app.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/router.rs`, `src-tauri/src/theming.rs`. Registration lines in `lib.rs` (append-only).

## Spec

- `app.rs` `setup()`:
  - Create the `Store` (or reuse T00's managed instance), and the art cache dir `app_cache_dir()/art`.
  - Spawn `ferric_media::run`.
  - Spawn the **emitter**: `store.subscribe()` → emit `ferric://state`, throttled to 30 per second (trailing edge, so the last state always goes out).
  - Spawn **theming**: watch for `track.art` changes; read the bytes (`File` → fs; `Url` → `reqwest` GET, 5 s timeout), call `ferric_theme::extract` in `spawn_blocking`, write `store.theme`. Drop stale results if the track changed meanwhile. Cache by `album_key` in an LRU of 32.
  - Leave a clearly marked spot (`// T10: spotify auth + poller here`) for T10.
- `router.rs`: `Router { controllers: Vec<Arc<dyn Controller>> }`; `execute` tries each in order until `Done`. If every controller returns `Unsupported`, return an error such as "Sign in to Spotify to use this".
- `commands.rs`: `get_state`, `send_command`, `set_queue_visible` (writes a `watch::Sender<bool>` kept in managed state for T10's poller), `open_spotify` (`open::that("spotify:")`).
- Logging: `tracing-subscriber` with `FERRIC_LOG` env filter (default `info`), plus a rolling file in `app_log_dir()`.

## Steps

1. Router + unit tests with fake controllers.
2. Emitter with throttle + test (`tokio::time::pause`).
3. Theming task + test with a fake extractor.
4. Commands + registration.

## Acceptance criteria

- [ ] `cargo tauri dev` with Spotify playing: the UI shows the current track within 1 s of launch and follows skips
- [ ] Play/pause/next/prev from the UI control Spotify (via `MediaController`)
- [ ] The theme changes with the album; switching between two albums quickly never leaves the theme of the wrong album
- [ ] `send_command(seek)` while signed out returns the "Sign in…" error
- [ ] Idle CPU < 1% and memory < 60 MB with the window visible (Task Manager; report the numbers)
- [ ] Router, throttle and stale-theme logic are unit-tested

## Out of scope

Spotify auth/poller wiring (T10), hotkeys/config (T11).
