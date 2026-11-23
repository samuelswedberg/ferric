use std::sync::Arc;

use ferric_core::{model::Mode, store::Store};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    AppHandle, Manager,
};
use tauri_plugin_opener::OpenerExt;

const MAIN_WINDOW: &str = "main";
const TRAY_ID: &str = "main";

struct TrayMenuState {
    visibility: MenuItem<tauri::Wry>,
    always_on_top: CheckMenuItem<tauri::Wry>,
    full: CheckMenuItem<tauri::Wry>,
    card: CheckMenuItem<tauri::Wry>,
    strip: CheckMenuItem<tauri::Wry>,
    start_with_windows: CheckMenuItem<tauri::Wry>,
    fade_when_idle: CheckMenuItem<tauri::Wry>,
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let window = app
        .get_webview_window(MAIN_WINDOW)
        .ok_or(tauri::Error::WindowNotFound)?;
    let current_mode = app.state::<Store>().get().mode;

    let visibility = MenuItem::with_id(app, "visibility", "Hide", true, None::<&str>)?;
    let always_on_top = CheckMenuItem::with_id(
        app,
        "always-on-top",
        "Always on top",
        true,
        window.is_always_on_top()?,
        None::<&str>,
    )?;
    let full = mode_item(app, "mode-full", "Full", current_mode == Mode::Full)?;
    let card = mode_item(app, "mode-card", "Card", current_mode == Mode::Card)?;
    let strip = mode_item(app, "mode-strip", "Strip", current_mode == Mode::Strip)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let open_config =
        MenuItem::with_id(app, "open-config", "Open config file", true, None::<&str>)?;
    let start_with_windows = CheckMenuItem::with_id(
        app,
        "start-with-windows",
        "Start with Windows",
        true,
        false,
        None::<&str>,
    )?;
    let fade_checked = app
        .try_state::<crate::config::ConfigManager>()
        .is_none_or(|config| config.current().fade_when_idle);
    let fade_when_idle = CheckMenuItem::with_id(
        app,
        "fade-when-idle",
        "Fade when idle",
        true,
        fade_checked,
        None::<&str>,
    )?;
    let config_separator = PredefinedMenuItem::separator(app)?;
    let sign_out = MenuItem::with_id(app, "sign-out", "Sign out", true, None::<&str>)?;
    let account_separator = PredefinedMenuItem::separator(app)?;
    let restart = MenuItem::with_id(app, "restart", "Restart Ferric", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &visibility,
            &always_on_top,
            &separator,
            &full,
            &card,
            &strip,
            &separator,
            &open_config,
            &start_with_windows,
            &fade_when_idle,
            &config_separator,
            &sign_out,
            &account_separator,
            &restart,
            &quit,
        ],
    )?;

    app.manage(TrayMenuState {
        visibility,
        always_on_top,
        full,
        card,
        strip,
        start_with_windows,
        fade_when_idle,
    });

    let tray = app
        .tray_by_id(TRAY_ID)
        .ok_or_else(|| tauri::Error::Anyhow(anyhow::anyhow!("configured tray icon is missing")))?;
    tray.set_menu(Some(menu))?;
    tray.on_menu_event(handle_menu_event);
    tray.on_tray_icon_event(|tray: &tauri::tray::TrayIcon<tauri::Wry>, event| {
        if matches!(
            event,
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
        ) {
            toggle_window(tray.app_handle());
        }
    });
    Ok(())
}

pub(crate) fn sync_mode(app: &AppHandle, mode: Mode) {
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu.full.set_checked(mode == Mode::Full);
        let _ = menu.card.set_checked(mode == Mode::Card);
        let _ = menu.strip.set_checked(mode == Mode::Strip);
    }
}

pub(crate) fn sync_visibility(app: &AppHandle, visible: bool) {
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu
            .visibility
            .set_text(if visible { "Hide" } else { "Show" });
    }
}

pub(crate) fn sync_autostart(app: &AppHandle, enabled: bool) {
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu.start_with_windows.set_checked(enabled);
    }
}

pub(crate) fn sync_fade_when_idle(app: &AppHandle, enabled: bool) {
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu.fade_when_idle.set_checked(enabled);
    }
}

fn mode_item(
    app: &AppHandle,
    id: &'static str,
    label: &'static str,
    checked: bool,
) -> tauri::Result<CheckMenuItem<tauri::Wry>> {
    CheckMenuItem::with_id(app, id, label, true, checked, None::<&str>)
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match action_for_menu_id(event.id().as_ref()) {
        Some(TrayAction::Visibility) => toggle_window(app),
        Some(TrayAction::AlwaysOnTop) => toggle_always_on_top(app),
        Some(TrayAction::Mode(mode)) => change_mode(app, mode),
        Some(TrayAction::OpenConfig) => open_config(app),
        Some(TrayAction::StartWithWindows) => toggle_start_with_windows(app),
        Some(TrayAction::FadeWhenIdle) => toggle_fade_when_idle(app),
        Some(TrayAction::SignOut) => sign_out(app),
        Some(TrayAction::Restart) => app.request_restart(),
        Some(TrayAction::Quit) => app.exit(0),
        None => {}
    }
}

#[derive(Debug, PartialEq)]
enum TrayAction {
    Visibility,
    AlwaysOnTop,
    Mode(Mode),
    OpenConfig,
    StartWithWindows,
    FadeWhenIdle,
    SignOut,
    Restart,
    Quit,
}

fn action_for_menu_id(id: &str) -> Option<TrayAction> {
    match id {
        "visibility" => Some(TrayAction::Visibility),
        "always-on-top" => Some(TrayAction::AlwaysOnTop),
        "mode-full" => Some(TrayAction::Mode(Mode::Full)),
        "mode-card" => Some(TrayAction::Mode(Mode::Card)),
        "mode-strip" => Some(TrayAction::Mode(Mode::Strip)),
        "open-config" => Some(TrayAction::OpenConfig),
        "start-with-windows" => Some(TrayAction::StartWithWindows),
        "fade-when-idle" => Some(TrayAction::FadeWhenIdle),
        "sign-out" => Some(TrayAction::SignOut),
        "restart" => Some(TrayAction::Restart),
        "quit" => Some(TrayAction::Quit),
        _ => None,
    }
}

fn open_config(app: &AppHandle) {
    let Some(config) = app.try_state::<crate::config::ConfigManager>() else {
        return;
    };
    let path = config.path().to_string_lossy().into_owned();
    if let Err(error) = app.opener().open_path(path, None::<&str>) {
        tracing::warn!(%error, "failed to open config.toml");
        crate::config::report_error(app, format!("Could not open config.toml: {error}"));
    }
}

fn toggle_start_with_windows(app: &AppHandle) {
    let Some(current) = app
        .try_state::<crate::config::ConfigManager>()
        .map(|config| config.current().start_with_windows)
    else {
        return;
    };
    if let Err(error) = crate::config::set_start_with_windows(app, !current) {
        tracing::warn!(%error, "failed to toggle start-with-Windows from tray");
        crate::config::report_error(app, format!("Could not update Start with Windows: {error}"));
        sync_autostart(app, current);
    }
}

fn toggle_fade_when_idle(app: &AppHandle) {
    let Some(current) = app
        .try_state::<crate::config::ConfigManager>()
        .map(|config| config.current().fade_when_idle)
    else {
        return;
    };
    if let Err(error) = crate::config::set_fade_when_idle(app, !current) {
        tracing::warn!(%error, "failed to toggle fade-when-idle from tray");
        crate::config::report_error(app, format!("Could not update Fade when idle: {error}"));
        sync_fade_when_idle(app, current);
    }
}

fn sign_out(app: &AppHandle) {
    let Some(auth) = app
        .try_state::<crate::auth::AuthState>()
        .map(|state| Arc::clone(state.inner()))
    else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        if let Err(error) = auth.sign_out().await {
            tracing::warn!(%error, "failed to sign out from the tray");
        }
    });
}

pub(crate) fn toggle_window(app: &AppHandle) {
    if let Err(error) = crate::window::toggle_window(app) {
        tracing::warn!(%error, "failed to toggle the main window");
    }
}

fn toggle_always_on_top(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };

    let enabled = !window.is_always_on_top().unwrap_or(true);
    if let Err(error) = window.set_always_on_top(enabled) {
        tracing::warn!(%error, "failed to change always-on-top state");
    } else if let Some(menu) = app.try_state::<TrayMenuState>() {
        let _ = menu.always_on_top.set_checked(enabled);
    }
}

fn change_mode(app: &AppHandle, mode: Mode) {
    if let Err(error) = crate::window::set_mode(app, mode) {
        tracing::warn!(%error, ?mode, "failed to change window mode");
    }
}

#[cfg(test)]
mod tests {
    use ferric_core::model::Mode;

    use super::{action_for_menu_id, TrayAction};

    #[test]
    fn tray_menu_ids_map_to_their_actions() {
        assert_eq!(
            action_for_menu_id("mode-card"),
            Some(TrayAction::Mode(Mode::Card))
        );
        assert_eq!(action_for_menu_id("restart"), Some(TrayAction::Restart));
        assert_eq!(action_for_menu_id("unknown"), None);
    }
}
