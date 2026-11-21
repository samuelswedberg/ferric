# T10: Sign-in, end to end

| Meta | |
| --- | --- |
| Wave | 3 |
| Depends on | T07, T08, T09 |
| Branch | `feat/t10-sign-in` |
| Milestone | M2 |
| PR title | `feat: first-run client ID setup and Spotify sign-in` |

## Goal

A new user can go from "Sign in to see your queue" to seeing their queue in about 2 minutes, guided by Ferric. After that it all works across restarts.

## Context

`docs/CONTRACTS.md` §1 (`Auth`), §5 (`Authenticator`, `poller::run`, `SpotifyController`), §6 (`auth_*` commands, owned here). T08 left a `// T10` marker in `app.rs`, and T09 left one in the queue's sign-in row.

## Owns

`ui/src/lib/components/onboarding/**`, `src-tauri/src/auth.rs`. Edits at the `// T10` markers in `src-tauri/src/app.rs` and `ui/src/lib/components/queue/*` (only there).

## Spec

**Backend (`auth.rs` + the marker in `app.rs`)**
- Build `Authenticator` with `KeyringTokenStore`, call `restore()` at startup (log errors, never crash).
- Spawn `poller::run(store, auth, queue_visible_rx)`.
- Push `SpotifyController` into the router **after** `MediaController`.
- Commands `auth_set_client_id`, `auth_sign_in`, `auth_sign_out`.

**Onboarding UI** (a panel that slides over the Full view; in Card/Strip mode it switches to Full first):
1. **Why**: one paragraph: "Ferric works without signing in. To see your queue and like songs, connect your own free Spotify developer app. It takes about 2 minutes, and your data never leaves your PC."
2. **Create the app**: numbered steps with a button "Open Spotify Developer Dashboard" (opens `https://developer.spotify.com/dashboard`). Show the exact redirect URI `http://127.0.0.1:8898/callback` with a copy button, and say to tick "Web API". (Check the current dashboard wording and update the copy to match.)
3. **Paste the client ID**: input + Save → `auth_set_client_id`. Show validation errors inline.
4. **Sign in**: button → `auth_sign_in`; a spinner while `auth.status === "signingIn"`; success → close the panel, and the queue appears.
- Error states for `auth.status === "error"`, and the port-in-use message with what to do.
- Tray/settings entry "Sign out" → `auth_sign_out`.
- If the account isn't Premium, a one-time note: "Seek, volume and queue jump need Spotify Premium. Play/pause/skip still work."

## Acceptance criteria

- [ ] A fresh Windows user profile (or a cleared Credential Manager entry) completes the whole flow, and the queue shows
- [ ] Restarting the app keeps the session; signing out returns to the "Sign in…" row, and the client ID is remembered
- [ ] A wrong client ID shows a clear error, not a hang
- [ ] With the session restored, the queue jump, like and seek commands work (Premium) or show the Premium note (free)
- [ ] The onboarding works in all three modes (switches to Full)
- [ ] Screen recording or GIF of the flow attached to the PR

## Out of scope

Changing T02/T07 internals (file issues instead), hotkeys.
