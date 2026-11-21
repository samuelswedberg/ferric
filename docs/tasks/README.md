# Tasks

Each `Txx-*.md` file is a self-contained brief you can hand to one agent: *"Do the task in docs/tasks/T01-media-watcher.md. Follow AGENTS.md."*

Tasks in the same wave can run **in parallel**, because they own separate files. Start a task only when everything in its **Depends on** list is merged to `main`.

## Waves

| Wave | Tasks (parallel within a wave) | Unblocks |
| --- | --- | --- |
| 0 | **T00** Bootstrap workspace + contracts (run alone, first) | everything |
| 1 | T01 Media watcher · T02 Spotify auth · T03 Theme engine · T04 UI Full view · T05 Window shell + tray · T06 CI release pipeline | wave 2 |
| 2 | T07 Spotify API + poller (needs T02) · T08 App wiring (needs T01 T03 T05) · T09 UI Card/Strip + queue (needs T04) | wave 3 |
| 3 | T10 Sign-in end to end (needs T07 T08 T09) · T11 Hotkeys, config, autostart (needs T08) | wave 4 |
| 4 | T12 v1.0 release hardening (needs T06 T10 T11) | v1.0.0 |

```
T00 ─┬─ T01 ──────────────┐
     ├─ T02 ── T07 ───────┼──────── T10 ──┐
     ├─ T03 ──────────────┤               │
     ├─ T05 ───────────── T08 ─┬──────────┤
     ├─ T04 ── T09 ────────────┼── (T10)  ├── T12
     └─ T06 ───────────────────┴── T11 ───┘
```

The shortest route to a usable v0.1.0 is T00 → T01 + T04 + T05 + T06 → T08.

## Who owns what

| Task | Branch | Owns (exclusive write) |
| --- | --- | --- |
| T00 | `feat/t00-bootstrap` | everything it creates; afterwards only `crates/ferric-core/**` |
| T01 | `feat/t01-media-watcher` | `crates/ferric-media/**` |
| T02 | `feat/t02-spotify-auth` | `crates/ferric-spotify/src/auth/**`, `crates/ferric-spotify/src/error.rs` |
| T03 | `feat/t03-theme-engine` | `crates/ferric-theme/**` |
| T04 | `feat/t04-ui-full-view` | `ui/src/lib/components/**` (except `modes/`, `onboarding/`), `ui/src/lib/styles/**`, `ui/src/App.svelte` |
| T05 | `feat/t05-window-shell` | `src-tauri/src/window.rs`, `src-tauri/src/tray.rs`, `src-tauri/tauri.conf.json` (`app.windows`, `app.trayIcon`), `src-tauri/icons/**` |
| T06 | `ci/t06-release-pipeline` | `.github/workflows/release.yml`, `.github/workflows/release-please.yml`, `release-please-config.json`, `.release-please-manifest.json`, `docs/RELEASING.md` |
| T07 | `feat/t07-spotify-api` | `crates/ferric-spotify/src/{api,poller,controller}.rs`, `crates/ferric-spotify/src/api/**` |
| T08 | `feat/t08-app-wiring` | `src-tauri/src/{app,commands,router,theming}.rs` |
| T09 | `feat/t09-ui-modes-queue` | `ui/src/lib/components/modes/**`, `ui/src/lib/components/queue/**`, `ui/src/App.svelte` |
| T10 | `feat/t10-sign-in` | `ui/src/lib/components/onboarding/**`, `src-tauri/src/auth.rs` |
| T11 | `feat/t11-hotkeys-config` | `src-tauri/src/{hotkeys,config,autostart}.rs` |
| T12 | `chore/t12-release-hardening` | `packaging/**`, `.github/workflows/release.yml`, `docs/RELEASING.md` |

**Shared files** (append-only: add your own lines, sorted, never reformat): root `Cargo.toml` `[workspace.dependencies]`, each crate's `Cargo.toml` `[dependencies]`, `ui/package.json` deps, `src-tauri/src/lib.rs` (`mod` lines and `.invoke_handler` entries), `src-tauri/capabilities/default.json`.

## Task file format

Every task has the same sections: **Meta** · **Goal** · **Context** · **Owns** · **Spec** · **Steps** · **Acceptance criteria** · **Out of scope**. The acceptance criteria are the definition of done. Every box must be checked, or explained in the PR.
