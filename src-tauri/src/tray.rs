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
    match event.id().as_ref() {
        "visibility" => toggle_window(app),
        "always-on-top" => toggle_always_on_top(app),
        "mode-full" => change_mode(app, Mode::Full),
        "mode-card" => change_mode(app, Mode::Card),
        "mode-strip" => change_mode(app, Mode::Strip),
        "open-config" => open_config(app),
        "start-with-windows" => toggle_start_with_windows(app),
        "fade-when-idle" => toggle_fade_when_idle(app),
        "sign-out" => sign_out(app),
        "quit" => app.exit(0),
        _ => {}
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
