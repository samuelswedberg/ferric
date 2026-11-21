# T11: Hotkeys, config file, start with Windows

| Meta | |
| --- | --- |
| Wave | 3 |
| Depends on | T08 |
| Branch | `feat/t11-hotkeys-config` |
| Milestone | M3 |
| PR title | `feat(app): global hotkeys, config file and autostart` |

## Goal

Make Ferric something you can leave running every day: global hotkeys, a config file for preferences, and an option to start with Windows.

## Context

Plugins: `tauri-plugin-global-shortcut`, `tauri-plugin-autostart`. Commands go through T08's `Router`. Mode changes go through T05's `window::set_mode`.

## Owns

`src-tauri/src/hotkeys.rs`, `src-tauri/src/config.rs`, `src-tauri/src/autostart.rs`. Registration lines in `lib.rs`/`app.rs` (append-only), tray menu entries via T05's public `tray` API (if it lacks a hook to add items, add a minimal `pub fn add_item` in a separate small commit, and say so in the PR).

## Spec

- Config: `%APPDATA%/dev.ferric.app/config.toml`, created with defaults and comments on first run. Changes on disk reload live (`notify` crate, 300 ms debounce). Invalid TOML → keep the old config and show a toast "config.toml has an error on line N".

```toml
grain = 0.2              # grain + dust overlay opacity, 0 to 0.5
start_with_windows = false
hide_when_idle = false   # hide the window after 30 s of nothing playing
idle_opacity = 0.4

[hotkeys]                # set any to "" to disable
play_pause = "Ctrl+Alt+Space"
next = "Ctrl+Alt+Right"
previous = "Ctrl+Alt+Left"
toggle_window = "Ctrl+Alt+F"
cycle_mode = "Ctrl+Alt+M"
```

- Hotkeys: register on start and re-register on config change. If one fails to register (taken by another app), log it and show a toast naming the hotkey; the others still work.
- `grain` → emit a `ferric://config` event; the UI sets the grain overlay's opacity from it (default 0.2, clamped 0 to 0.5).
- Idle: after 30 s of `Paused`/`Stopped`, set the window opacity to `idle_opacity` (or hide it if `hide_when_idle`); restore on play.
- Tray: "Open config file" and "Start with Windows ✓".

## Acceptance criteria

- [ ] Each default hotkey works while another app has focus
- [ ] Editing `config.toml` changes the theme and hotkeys without a restart; a broken file shows the toast and keeps working
- [ ] "Start with Windows" survives a reboot (or a sign-out and sign-in)
- [ ] The idle fade and idle hide behave as configured
- [ ] Unit tests for config parsing, defaults and the error-line reporting

## Out of scope

A settings window UI (the config file is enough for v1), lyrics.
