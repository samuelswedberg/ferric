use ferric_core::{command::Command, model::Mode, store::Store};
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
    let display = shortcut.to_owned();
    let result = app
        .global_shortcut()
        .on_shortcut(shortcut, move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                dispatch(app.clone(), action);
            }
        });
    if let Err(error) = result {
        tracing::warn!(hotkey = %display, %error, "failed to register global hotkey");
        crate::config::report_error(
            app,
            format!("Could not register hotkey {display}: {error}"),
        );
    }
}

fn dispatch(app: AppHandle, action: Action) {
    match action {
        Action::PlayPause => execute(app, Command::PlayPause),
        Action::Next => execute(app, Command::Next),
        Action::Previous => execute(app, Command::Previous),
        Action::ToggleWindow => crate::tray::toggle_window(&app),
        Action::CycleMode => {
            let next = next_mode(app.state::<Store>().get().mode);
            if let Err(error) = crate::window::set_mode(&app, next) {
                tracing::warn!(%error, "global hotkey could not change window mode");
                crate::config::report_error(&app, error.to_string());
            }
        }
    }
}

fn next_mode(mode: Mode) -> Mode {
    match mode {
        Mode::Full => Mode::Card,
        Mode::Card => Mode::Strip,
        Mode::Strip => Mode::Full,
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

    use ferric_core::model::Mode;
    use tauri_plugin_global_shortcut::Shortcut;

    use super::next_mode;
    use crate::config::Hotkeys;

    #[test]
    fn mode_cycle_visits_every_mode() {
        assert_eq!(next_mode(Mode::Full), Mode::Card);
        assert_eq!(next_mode(Mode::Card), Mode::Strip);
        assert_eq!(next_mode(Mode::Strip), Mode::Full);
    }

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
            Shortcut::from_str(shortcut)
                .unwrap_or_else(|error| panic!("default hotkey {shortcut:?} failed to parse: {error}"));
        }
    }
}
