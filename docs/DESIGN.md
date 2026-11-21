# Ferric design (summary)

This is the in-repo summary of the design doc. When the two disagree, update this file in a `docs:` PR.

## What it is

An always-on-top Spotify miniplayer for Windows. Ferric is a **remote control** for the Spotify desktop app: it never plays audio itself.

- It **works with zero setup**: now playing and play/pause/next/previous come from Windows media controls (SMTC). No login needed.
- **Signing in** (PKCE, with the user's own Spotify client ID) adds the queue, likes, seek, volume and queue jump.

## Goals

- The track updates within 200 ms of Spotify changing tracks.
- Under 60 MB of memory and close to 0% CPU when idle.
- Colors come from the album art: a blurred cover background, an accent color, cross-fades between tracks.
- Three sizes: Full 360×520, Card 360×120, Strip 360×36.
- Open source (MIT OR Apache-2.0). A person can install it and have it working in under a minute.

## Non-goals

- Playing audio, browsing the library, managing playlists.
- macOS/Linux in v1 (but keep platform code behind `cfg`).
- A shared Spotify client ID. Each user brings their own, so developer-mode limits never apply.

## Architecture

```
 Spotify desktop app ──(SMTC events)──▶ ferric-media ──┐
                                                        ├──▶ ferric-core::Store (PlayerState) ──(Tauri event)──▶ ui/
 Spotify Web API ◀──(rspotify, polling)──▶ ferric-spotify ┘            ▲                                        │
                                                                       │                                        │
                                       ferric-theme (accent + bg) ─────┘          commands ◀────────────────────┘
                                                                                     │
                                                       src-tauri router ─▶ Controller (media first, then spotify)
```

| Crate / dir | Job |
| --- | --- |
| `crates/ferric-core` | Shared types, the `Store`, the `Controller` trait. No I/O. **The contracts.** |
| `crates/ferric-media` | Windows SMTC watcher + `MediaController` (play/pause/next/prev). |
| `crates/ferric-spotify` | PKCE auth, token storage, Web API client, queue poller, `SpotifyController`. |
| `crates/ferric-theme` | Album art → `Theme` (accent, text colors, blurred background file). |
| `src-tauri` | The app: window, tray, hotkeys, config, wiring, command router, IPC. |
| `ui` | Svelte 5 + TS front end. It only draws `PlayerState` and calls commands. |

Data flows one way: sources write to the `Store`, the app emits each new state to the UI, and the UI sends `Command`s back. The UI never talks to Spotify.

## Spotify Web API (verify against current Spotify docs before building)

- Auth: Authorization Code with PKCE. Redirect `http://127.0.0.1:8898/callback` (loopback IP; `localhost` is not accepted).
- Scopes: `user-read-playback-state user-read-currently-playing user-modify-playback-state user-library-read user-library-modify`.
- Refresh token stored in Windows Credential Manager via `keyring`.
- Endpoints: `GET /me/player`, `GET /me/player/queue`, `POST /me/player/next|previous`, `PUT /me/player/seek|volume`, `PUT|DELETE /me/tracks`, `GET /me/tracks/contains`.
- Playback control through the Web API needs Premium. Ferric prefers SMTC for play/pause/next/prev, so free accounts keep those.
- Polling: on every track change, plus every 15 s while the queue is visible. Nothing while hidden. On 429, wait for `Retry-After`, then back off further.
- Queue jump: there's no API call for it, so Ferric sends `next` N times. Allowed only for N ≤ 10.

## Visual design

- Background: the cover, downscaled, blurred (about 60 px at display size), darkened to 35% brightness.
- Accent: the most vivid of the cover's main colors, adjusted to at least 4.5:1 contrast against the background.
- Art: 216 px centered in Full (leaving room for the queue), 88 px in Card, 24 px in Strip, with 12 / 8 / 4 px rounded corners.
- Mockups and exact measurements: [docs/design/](design/README.md).
- Type: Segoe UI Variable. Title 17/600, artist 13/400 at 70% opacity, queue rows 13 px.
- Motion: 300 ms cross-fade on track change, 200 ms resize between sizes. No animation when the user has asked Windows for reduced motion.
- Themes: Cover (default), Mono (greyscale), Plain (solid dark).

## Milestones

| Milestone | Tasks | Ships as |
| --- | --- | --- |
| M0 Now playing, no login | T00 T01 T04 T05 T06 T08 | v0.1.0 pre-release |
| M1 Looks right | T03 T09 | v0.2.0 |
| M2 Signed in | T02 T07 T10 | v0.3.0 |
| M3 Daily driver | T11 T12 | v1.0.0 |
| M4 Extras | lyrics, device picker, macOS look | later |
