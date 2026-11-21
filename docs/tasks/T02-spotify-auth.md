# T02: Spotify auth (PKCE, bring-your-own client ID)

| Meta | |
| --- | --- |
| Wave | 1 |
| Depends on | T00 |
| Branch | `feat/t02-spotify-auth` |
| Milestone | M2 |
| PR title | `feat(spotify): PKCE sign-in with user-supplied client ID` |

## Goal

Let a user sign in to Spotify with **their own** client ID, using Authorization Code + PKCE, and stay signed in across restarts.

## Context

`docs/CONTRACTS.md` §1 (`Auth`) and §5 (`auth` module). Background: Spotify's developer mode caps each app at a few named users, so Ferric never ships a shared client ID. **Before writing code, check the current Spotify Web API authorization docs** for the PKCE flow, the redirect URI rules (loopback IP, not `localhost`) and the scope names, and note any differences in the PR.

Crates: `rspotify` (features `client-reqwest`, `reqwest-rustls-tls`; no default features), `keyring` 3.x, `tiny_http` or `axum` for the loopback listener (pick the lighter one), `open` (to launch the browser), `url`.

## Owns

`crates/ferric-spotify/src/auth/**`, `crates/ferric-spotify/src/error.rs`

## Spec

- Redirect URI: `http://127.0.0.1:8898/callback`. If port 8898 is taken, fail with a clear `AuthError::PortInUse`. Don't pick a random port: the redirect URI must match the one registered with Spotify exactly.
- Scopes: exactly the list in `docs/DESIGN.md`.
- `set_client_id`: check it's 32 hex characters, save it through `TokenStore` (clearing any refresh token), and set `auth = SignedOut`.
- `sign_in`: set `auth = SigningIn`, open the browser at the authorize URL, accept **one** callback, check `state`, exchange the code, save the refresh token, fetch `/me`, then set `auth = SignedIn { display_name, premium: product == "premium" }`. Time out after 3 minutes (`auth = SignedOut`). The callback page says "You're signed in to Ferric. You can close this tab." in plain HTML.
- `restore`: at startup, load from `TokenStore`; no client ID → `NoClientId`; no token → `SignedOut`; otherwise refresh. On `invalid_grant`, clear the token and set `SignedOut`.
- Refresh access tokens automatically before they expire (rspotify's token refresh, or your own).
- `sign_out`: clear the refresh token (keep the client ID), set `SignedOut`.
- `KeyringTokenStore`: Windows Credential Manager, service `ferric`, user `spotify`, value = JSON of `StoredAuth`.
- Never log tokens or the client ID (log `client_id[..4]…` at most).

## Steps

1. `TokenStore` trait + `KeyringTokenStore` + an in-memory `MemoryTokenStore` for tests.
2. Loopback callback server (single request, then shut down).
3. `Authenticator` state machine, with each transition written to `store.auth`.
4. Example: `cargo run -p ferric-spotify --example sign_in -- <client_id>` signs in and prints the display name and whether it's Premium.

## Acceptance criteria

- [ ] With a real client ID (registered with the redirect URI above), the example opens the browser, completes sign-in and prints the display name
- [ ] Running the example again without signing in restores the session through the stored refresh token
- [ ] A wrong `state` in the callback is rejected; a timeout returns to `SignedOut`
- [ ] Unit tests (no network): client ID validation, state transitions with `MemoryTokenStore`, the `restore` branches
- [ ] Nothing secret appears in the `tracing` output at `trace` level
- [ ] Clippy clean

## Out of scope

Calling player/queue endpoints (T07), the onboarding UI (T10), wiring into the app (T10).
