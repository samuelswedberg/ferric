use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};

use ferric_core::{model::Playback, store::Store};
use notify::{RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tokio::sync::watch;

const CONFIG_EVENT: &str = "ferric://config";
const ERROR_EVENT: &str = "ferric://error";
const MAIN_WINDOW: &str = "main";
const IDLE_AFTER: Duration = Duration::from_secs(30);
const RELOAD_DEBOUNCE: Duration = Duration::from_millis(300);

const DEFAULT_CONFIG_TOML: &str = r#"# Ferric preferences. Changes are applied without restarting.
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
"#;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Config {
    pub grain: f64,
    pub start_with_windows: bool,
    pub hide_when_idle: bool,
    pub idle_opacity: f64,
    pub hotkeys: Hotkeys,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grain: 0.2,
            start_with_windows: false,
            hide_when_idle: false,
            idle_opacity: 0.4,
            hotkeys: Hotkeys::default(),
        }
    }
}

impl Config {
    fn normalized(mut self) -> Self {
        self.grain = self.grain.clamp(0.0, 0.5);
        self.idle_opacity = self.idle_opacity.clamp(0.0, 1.0);
        self
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Hotkeys {
    pub play_pause: String,
    pub next: String,
    pub previous: String,
    pub toggle_window: String,
    pub cycle_mode: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            play_pause: "Ctrl+Alt+Space".to_owned(),
            next: "Ctrl+Alt+Right".to_owned(),
            previous: "Ctrl+Alt+Left".to_owned(),
            toggle_window: "Ctrl+Alt+F".to_owned(),
            cycle_mode: "Ctrl+Alt+M".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPayload {
    grain: f64,
    errors: Vec<String>,
}

#[derive(Debug, PartialEq)]
struct ParseError {
    line: usize,
}

pub struct ConfigManager {
    path: PathBuf,
    current: RwLock<Config>,
    changes: watch::Sender<Config>,
    pending_errors: Mutex<Vec<String>>,
    ui_ready: AtomicBool,
}

impl ConfigManager {
    fn new(path: PathBuf, config: Config, initial_error: Option<String>) -> Self {
        let (changes, _) = watch::channel(config.clone());
        Self {
            path,
            current: RwLock::new(config),
            changes,
            pending_errors: Mutex::new(initial_error.into_iter().collect()),
            ui_ready: AtomicBool::new(false),
        }
    }

    pub fn current(&self) -> Config {
        self.current
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn replace(&self, config: Config) -> bool {
        let mut current = self
            .current
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *current == config {
            return false;
        }
        *current = config.clone();
        self.changes.send_replace(config);
        true
    }

    fn subscribe(&self) -> watch::Receiver<Config> {
        self.changes.subscribe()
    }

    fn queue_error_until_ui_is_ready(&self, message: String) -> bool {
        if self.ui_ready.load(Ordering::Acquire) {
            return false;
        }
        self.pending_errors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(message);
        true
    }

    fn ready_payload(&self) -> ConfigPayload {
        self.ui_ready.store(true, Ordering::Release);
        let errors = std::mem::take(
            &mut *self
                .pending_errors
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        ConfigPayload {
            grain: self.current().grain,
            errors,
        }
    }
}

pub fn setup(app: &mut tauri::App) -> anyhow::Result<()> {
    let directory = app.path().app_config_dir()?;
    fs::create_dir_all(&directory)?;
    let path = directory.join("config.toml");
    if !path.exists() {
        fs::write(&path, DEFAULT_CONFIG_TOML)?;
    }

    let (config, initial_error) = match load(&path) {
        Ok(config) => (config, None),
        Err(error) => {
            let message = config_error_message(error.line);
            tracing::warn!(path = %path.display(), %message, "invalid config; using defaults");
            (Config::default(), Some(message))
        }
    };

    app.manage(ConfigManager::new(path.clone(), config.clone(), initial_error));
    crate::hotkeys::apply(app.handle(), &config.hotkeys);
    crate::autostart::apply(app.handle(), config.start_with_windows);
    start_watcher(app.handle().clone(), path)?;
    start_idle_watcher(app.handle().clone(), app.state::<Store>().inner().clone());
    Ok(())
}

#[tauri::command]
pub async fn get_config(config: State<'_, ConfigManager>) -> Result<ConfigPayload, String> {
    Ok(config.ready_payload())
}

pub fn set_start_with_windows(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.state::<ConfigManager>();
    let mut config = manager.current();
    config.start_with_windows = enabled;
    let contents = toml::to_string_pretty(&config).map_err(|error| error.to_string())?;
    fs::write(manager.path(), contents).map_err(|error| error.to_string())?;
    drop(manager);
    apply(app, config);
    Ok(())
}

pub fn report_error(app: &AppHandle, message: impl Into<String>) {
    let message = message.into();
    if let Some(config) = app.try_state::<ConfigManager>() {
        if config.queue_error_until_ui_is_ready(message.clone()) {
            return;
        }
    }
    if let Err(error) = app.emit(ERROR_EVENT, &message) {
        tracing::warn!(%error, "failed to emit UI error");
    }
}

fn start_watcher(app: AppHandle, path: PathBuf) -> anyhow::Result<()> {
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(
        move |event: notify::Result<notify::Event>| {
            let _ = sender.send(event);
        },
    )?;
    let directory = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("config path has no parent"))?;
    watcher.watch(directory, RecursiveMode::NonRecursive)?;

    std::thread::Builder::new()
        .name("ferric-config".to_owned())
        .spawn(move || {
            let _watcher = watcher;
            while let Ok(event) = receiver.recv() {
                if !relevant_event(event, &path, &app) {
                    continue;
                }

                let mut deadline = Instant::now() + RELOAD_DEBOUNCE;
                loop {
                    let timeout = deadline.saturating_duration_since(Instant::now());
                    match receiver.recv_timeout(timeout) {
                        Ok(event) if relevant_event(event, &path, &app) => {
                            deadline = Instant::now() + RELOAD_DEBOUNCE;
                        }
                        Ok(_) => {}
                        Err(mpsc::RecvTimeoutError::Timeout) => break,
                        Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                }
                reload(&app, &path);
            }
        })?;
    Ok(())
}

fn relevant_event(
    event: notify::Result<notify::Event>,
    path: &Path,
    app: &AppHandle,
) -> bool {
    match event {
        Ok(event) => event.paths.iter().any(|changed| changed == path),
        Err(error) => {
            tracing::warn!(%error, "config file watcher error");
            report_error(app, format!("Could not watch config.toml: {error}"));
            false
        }
    }
}

fn reload(app: &AppHandle, path: &Path) {
    match load(path) {
        Ok(config) => apply(app, config),
        Err(error) => {
            let message = config_error_message(error.line);
            tracing::warn!(path = %path.display(), %message, "invalid config; keeping previous settings");
            report_error(app, message);
        }
    }
}

fn apply(app: &AppHandle, config: Config) {
    if !app.state::<ConfigManager>().replace(config.clone()) {
        return;
    }
    if let Err(error) = app.emit(
        CONFIG_EVENT,
        ConfigPayload {
            grain: config.grain,
            errors: Vec::new(),
        },
    ) {
        tracing::warn!(%error, "failed to emit config");
    }
    crate::hotkeys::apply(app, &config.hotkeys);
    crate::autostart::apply(app, config.start_with_windows);
}

fn load(path: &Path) -> Result<Config, ParseError> {
    let contents = fs::read_to_string(path).map_err(|_| ParseError { line: 1 })?;
    parse(&contents)
}

fn parse(contents: &str) -> Result<Config, ParseError> {
    toml::from_str::<Config>(contents)
        .map(Config::normalized)
        .map_err(|error| ParseError {
            line: error
                .span()
                .map(|span| contents[..span.start].lines().count().max(1))
                .unwrap_or(1),
        })
}

fn config_error_message(line: usize) -> String {
    format!("config.toml has an error on line {line}")
}

#[derive(Clone, Copy, PartialEq)]
enum IdleEffect {
    None,
    Faded,
    Hidden,
}

fn start_idle_watcher(app: AppHandle, store: Store) {
    let mut player = store.subscribe();
    let mut config = app.state::<ConfigManager>().subscribe();
    tauri::async_runtime::spawn(async move {
        run_idle_watcher(&app, &mut player, &mut config).await;
    });
}

async fn run_idle_watcher(
    app: &AppHandle,
    player: &mut watch::Receiver<ferric_core::model::PlayerState>,
    config: &mut watch::Receiver<Config>,
) {
    let mut idle_since = None;
    let mut effect = IdleEffect::None;

    loop {
        let playback = player.borrow().playback;
        if playback == Playback::Playing {
            idle_since = None;
            restore_from_idle(app, &mut effect);
            tokio::select! {
                changed = player.changed() => if changed.is_err() { return },
                changed = config.changed() => if changed.is_err() { return },
            }
            continue;
        }

        let since = *idle_since.get_or_insert_with(tokio::time::Instant::now);
        let deadline = since + IDLE_AFTER;
        if tokio::time::Instant::now() >= deadline {
            apply_idle(app, &config.borrow().clone(), &mut effect);
            tokio::select! {
                changed = player.changed() => if changed.is_err() { return },
                changed = config.changed() => if changed.is_err() { return },
            }
        } else {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => {},
                changed = player.changed() => if changed.is_err() { return },
                changed = config.changed() => if changed.is_err() { return },
            }
        }
    }
}

fn apply_idle(app: &AppHandle, config: &Config, effect: &mut IdleEffect) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if config.hide_when_idle {
        if *effect == IdleEffect::Faded {
            set_window_opacity(&window, 1.0);
        }
        if window.is_visible().unwrap_or(false) {
            if let Err(error) = window.hide() {
                tracing::warn!(%error, "failed to hide idle window");
                return;
            }
            crate::tray::sync_visibility(app, false);
            *effect = IdleEffect::Hidden;
        }
    } else {
        if *effect == IdleEffect::Hidden {
            if let Err(error) = window.show() {
                tracing::warn!(%error, "failed to show idle window");
                return;
            }
            crate::tray::sync_visibility(app, true);
        }
        if set_window_opacity(&window, config.idle_opacity) {
            *effect = IdleEffect::Faded;
        }
    }
}

fn restore_from_idle(app: &AppHandle, effect: &mut IdleEffect) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    match *effect {
        IdleEffect::Faded => {
            set_window_opacity(&window, 1.0);
        }
        IdleEffect::Hidden => {
            if let Err(error) = window.show() {
                tracing::warn!(%error, "failed to restore idle window");
            } else {
                crate::tray::sync_visibility(app, true);
            }
        }
        IdleEffect::None => {}
    }
    *effect = IdleEffect::None;
}

#[cfg(windows)]
fn set_window_opacity(window: &WebviewWindow, opacity: f64) -> bool {
    use windows::Win32::{
        Foundation::COLORREF,
        UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE,
            LWA_ALPHA, WS_EX_LAYERED,
        },
    };

    let hwnd = match window.hwnd() {
        Ok(hwnd) => hwnd,
        Err(error) => {
            tracing::warn!(%error, "failed to get window handle for idle opacity");
            return false;
        }
    };
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    // SAFETY: hwnd belongs to the live Tauri window. These calls only update its
    // extended style and uniform alpha value.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
        match SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(%error, "failed to change idle window opacity");
                false
            }
        }
    }
}

#[cfg(not(windows))]
fn set_window_opacity(_window: &WebviewWindow, _opacity: f64) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{config_error_message, parse, Config, Hotkeys};

    #[test]
    fn defaults_match_the_documented_config() {
        assert_eq!(
            Config::default(),
            Config {
                grain: 0.2,
                start_with_windows: false,
                hide_when_idle: false,
                idle_opacity: 0.4,
                hotkeys: Hotkeys {
                    play_pause: "Ctrl+Alt+Space".to_owned(),
                    next: "Ctrl+Alt+Right".to_owned(),
                    previous: "Ctrl+Alt+Left".to_owned(),
                    toggle_window: "Ctrl+Alt+F".to_owned(),
                    cycle_mode: "Ctrl+Alt+M".to_owned(),
                },
            }
        );
    }

    #[test]
    fn parses_partial_config_with_defaults_and_clamps_visual_values() {
        let parsed = parse(
            r#"grain = 0.9
hide_when_idle = true

[hotkeys]
next = ""
"#,
        )
        .expect("valid config");

        assert_eq!(parsed.grain, 0.5);
        assert!(parsed.hide_when_idle);
        assert_eq!(parsed.idle_opacity, 0.4);
        assert_eq!(parsed.hotkeys.next, "");
        assert_eq!(parsed.hotkeys.play_pause, "Ctrl+Alt+Space");
    }

    #[test]
    fn reports_the_toml_error_line() {
        let error = parse(
            r#"grain = 0.2
start_with_windows = false
idle_opacity = nope
"#,
        )
        .expect_err("invalid TOML");

        assert_eq!(error.line, 3);
        assert_eq!(
            config_error_message(error.line),
            "config.toml has an error on line 3"
        );
    }
}
