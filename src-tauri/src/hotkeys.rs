use ferric_core::command::Command;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::{config::Hotkeys, router::Router};

#[derive(Clone, Copy, Debug)]
enum Action {
    PlayPause,
    Next,
    Previous,
    ToggleWindow,
    CycleMode,
}

pub fn apply(app: &AppHandle, hotkeys: &Hotkeys) {
    if let Err(error) = app.global_shortcut().unregister_all() {
        tracing::warn!(%error, "failed to unregister old global hotkeys");
        crate::config::report_error(app, format!("Could not reload hotkeys: {error}"));
    }

    register(app, &hotkeys.play_pause, Action::PlayPause);
    register(app, &hotkeys.next, Action::Next);
    register(app, &hotkeys.previous, Action::Previous);
    register(app, &hotkeys.toggle_window, Action::ToggleWindow);
    register(app, &hotkeys.cycle_mode, Action::CycleMode);
}

fn register(app: &AppHandle, shortcut: &str, action: Action) {
    if shortcut.trim().is_empty() {
        return;
    }
    let hotkey_display = shortcut.to_owned();
    let result = app
        .global_shortcut()
        .on_shortcut(shortcut, move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                dispatch(app.clone(), action);
            }
        });
    if let Err(error) = result {
        tracing::warn!(hotkey = %hotkey_display, %error, "failed to register global hotkey");
        crate::config::report_error(
            app,
            format!("Could not register hotkey {hotkey_display}: {error}"),
        );
    }
}

fn dispatch(app: AppHandle, action: Action) {
    match action {
        Action::PlayPause => execute(app, Command::PlayPause),
        Action::Next => execute(app, Command::Next),
        Action::Previous => execute(app, Command::Previous),
        Action::ToggleWindow => run_window_action(&app, crate::window::toggle_window),
        Action::CycleMode => {
            run_window_action(&app, crate::window::cycle_mode);
        }
    }
}

fn run_window_action(app: &AppHandle, action: fn(&AppHandle) -> tauri::Result<()>) {
    if let Err(error) = action(app) {
        tracing::warn!(%error, "global hotkey window action failed");
        crate::config::report_error(app, error.to_string());
    }
}

fn execute(app: AppHandle, command: Command) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = app.state::<Router>().execute(&command).await {
            crate::config::report_error(&app, error);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use tauri_plugin_global_shortcut::Shortcut;

    use crate::config::Hotkeys;

    #[test]
    fn default_hotkeys_parse_as_valid_shortcuts() {
        let defaults = Hotkeys::default();
        for shortcut in [
            &defaults.play_pause,
            &defaults.next,
            &defaults.previous,
            &defaults.toggle_window,
            &defaults.cycle_mode,
        ] {
            Shortcut::from_str(shortcut).unwrap_or_else(|error| {
                panic!("default hotkey {shortcut:?} failed to parse: {error}")
            });
        }
    }
}
