# T05: Window shell and tray

| Meta | |
| --- | --- |
| Wave | 1 |
| Depends on | T00 |
| Branch | `feat/t05-window-shell` |
| Milestone | M0 |
| PR title | `feat(app): window modes, position memory and tray` |

## Goal

Make the window behave like a miniplayer: borderless, always on top, draggable, snapping to screen edges, remembering its position, switching between three sizes, and with a tray icon.

## Context

`docs/DESIGN.md` → User experience. `docs/CONTRACTS.md` §1 (`Mode`, written by this task) and §6 (`set_mode`). Plugins: `tauri-plugin-window-state`, and Tauri 2's built-in `tray-icon` feature.

## Owns

`src-tauri/src/window.rs`, `src-tauri/src/tray.rs`, the `app.windows` and `app.trayIcon` sections of `src-tauri/tauri.conf.json`, `src-tauri/icons/**`. Register in `lib.rs` (append-only).

## Spec

- Sizes: Full 360×520, Card 360×120, Strip 360×36 (logical px). `set_mode(mode)` resizes **keeping the top-left corner fixed**, but clamps to the current monitor's work area. Write `store.mode` (the store is reached via `app.state::<ferric_core::store::Store>()`; T00 registers it as managed state).
- Window: `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`, `resizable: false`, `shadow: true`. On Windows 11, apply Mica or Acrylic with `window-vibrancy` if available; otherwise a solid fallback.
- Snap: on move end, if within 16 px of a work-area edge, snap to it.
- Position and mode are restored at startup (`tauri-plugin-window-state`, plus the mode saved alongside). If the saved position is off-screen (monitor unplugged), center on the primary monitor.
- Close button / `hide()` hides the window; it doesn't quit.
- Tray icon (left-click toggles show/hide), menu: Show/Hide · Always on top ✓ · Full / Card / Strip (radio) · Quit.
- Icons: a placeholder Ferric icon (a simple rust-orange circle with a white "F" is fine); generate every size with `cargo tauri icon`.

## Steps

1. `window.rs`: `pub fn set_mode(app: &AppHandle, mode: Mode) -> tauri::Result<()>`, the `#[tauri::command] set_mode`, the snap handler, off-screen recovery.
2. `tray.rs`: `pub fn build(app: &AppHandle) -> tauri::Result<()>`.
3. Register both in `lib.rs`.

## Acceptance criteria

- [ ] `cargo tauri dev`: the window has no frame, stays above other windows, and can be dragged by the title strip
- [ ] Calling `set_mode` from the devtools console (`__TAURI__.core.invoke('set_mode', {mode:'card'})`) resizes correctly, with the top-left fixed
- [ ] Dragging near a screen edge snaps to it
- [ ] Restarting the app restores the position and mode; with the saved position set off-screen by hand, it recovers to the center
- [ ] The tray menu works for every item; the close button hides, and Quit exits
- [ ] Works at 100%, 125% and 150% display scaling (note in the PR what you tested)

## Out of scope

Global hotkeys and autostart (T11), animated resize (T09), the UI content.
