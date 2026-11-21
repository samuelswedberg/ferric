# T07: Spotify API client, poller and controller

| Meta | |
| --- | --- |
| Wave | 2 |
| Depends on | T00, T02 |
| Branch | `feat/t07-spotify-api` |
| Milestone | M2 |
| PR title | `feat(spotify): queue poller and Web API controller` |

## Goal

Once the user is signed in, keep `queue`, `liked`, `volume_pct`, `position` and the track's Spotify details up to date, and implement `SpotifyController` for every `Command`.

## Context

`docs/CONTRACTS.md` §1 (see the "Who writes which field" table: these fields are yours), §2, §3, §5. `docs/DESIGN.md` → Spotify Web API (endpoints, polling budget, queue jump). Get the client from `Authenticator::client()` (T02). **Check the current Spotify Web API reference** for each endpoint before using it.

## Owns

`crates/ferric-spotify/src/api.rs` or `src/api/**`, `src/poller.rs`, `src/controller.rs`

## Spec

**Poller** (`poller::run(store, auth, visible)`):
- Wakes on: a track change in the store (title/artist differ from the last poll), `visible` turning true, a 15 s tick **while visible and signed in**, and 1 s after any command it executed. Debounce to at most 1 call per endpoint per 2 s.
- Calls `GET /me/player` and, when visible, `GET /me/player/queue`, plus `GET /me/tracks/contains?ids=` when the track id changes.
- Enrich the track only if the title and first artist match the store's current track (case-insensitive, trimmed). Prefer the largest cover ≤ 640 px as `Art::Url`.
- Signed out or no client → no calls; set `queue/liked/volume_pct = None`.
- 429: sleep `Retry-After` (default 5 s), then double the interval up to 60 s; reset after a success. 401: ask the authenticator to refresh once, then give up until the next trigger.
- Never more than ~12 requests per minute in steady state (log a warning if exceeded).

**Controller** (`SpotifyController::execute`):
- `PlayPause` (via `GET` state → `pause`/`resume`), `Next`, `Previous`, `Seek`, `SetVolume`, `SetLiked` (`PUT`/`DELETE /me/tracks`), `JumpTo { i }` (reject `i > 9` with `Other("too far")`; otherwise `next` i+1 times, 150 ms apart).
- Not signed in → `NotSignedIn`. 403 on player endpoints → `PremiumRequired`. 404 "no active device" → `NotRunning`.
- After a success, update the store optimistically (`liked`, `volume_pct`) before the next poll confirms.

## Steps

1. A thin `api` module over rspotify (or raw `reqwest` for endpoints rspotify lacks), with typed responses.
2. Pure functions for the mapping and enrichment, with unit tests built on recorded JSON fixtures (`tests/fixtures/*.json`; scrub user ids).
3. The poller loop with `tokio::select!` over its triggers.
4. The controller.
5. Example: `cargo run -p ferric-spotify --example live`: restores auth (T02), runs the poller with visible=true and prints the queue.

## Acceptance criteria

- [ ] The example prints the current queue (titles + artists) within 2 s of starting, and again after a track skip
- [ ] Liking a song in the Spotify app shows up as `liked=true` within one poll cycle
- [ ] Every `Command` works with a Premium account; with a free account, the player commands return `PremiumRequired` (note in the PR which you could test)
- [ ] The rate-limit backoff is unit-tested with a fake clock (`tokio::time::pause`)
- [ ] Enrichment mismatch (different title) leaves the track untouched (test)
- [ ] No request is made while signed out or not visible (test with a counting mock)

## Out of scope

Sign-in UI, wiring into the app (T10), lyrics.
