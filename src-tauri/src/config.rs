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
use notify::{EventKind, RecursiveMode, Watcher};
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
        self.grain = if self.grain.is_nan() {
            0.2
        } else {
            self.grain.clamp(0.0, 0.5)
        };
        self.idle_opacity = if self.idle_opacity.is_nan() {
            0.4
        } else {
            self.idle_opacity.clamp(0.0, 1.0)
        };
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

    fn replace(&self, config: Config) -> Option<Config> {
        let mut current = self
            .current
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *current == config {
            return None;
        }
        let previous = std::mem::replace(&mut *current, config.clone());
        self.changes.send_replace(config);
        Some(previous)
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

    app.manage(ConfigManager::new(
        path.clone(),
        config.clone(),
        initial_error,
    ));
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
    let (mut config, path) = {
        let manager = app.state::<ConfigManager>();
        (manager.current(), manager.path().to_owned())
    };
    config.start_with_windows = enabled;
    let contents = fs::read_to_string(&path)
        .ok()
        .map(|existing| update_start_with_windows_in_toml(&existing, enabled))
        .filter(|updated| {
            parse(updated)
                .map(|parsed| parsed.start_with_windows == enabled)
                .unwrap_or(false)
        })
        .map(Ok)
        .unwrap_or_else(|| toml::to_string_pretty(&config).map_err(|error| error.to_string()))?;
    fs::write(path, contents).map_err(|error| error.to_string())?;
    apply(app, config);
    Ok(())
}

fn update_start_with_windows_in_toml(existing: &str, enabled: bool) -> String {
    let line_ending = if existing.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines: Vec<String> = existing.lines().map(str::to_owned).collect();
    let mut first_section = None;
    let mut replaced = false;

    for (index, line) in lines.iter_mut().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            first_section = Some(index);
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("start_with_windows") {
            let after_key = rest.trim_start();
            if let Some(after_eq) = after_key.strip_prefix('=') {
                let comment = after_eq
                    .find('#')
                    .map(|idx| format!(" {}", after_eq[idx..].trim_end()))
                    .unwrap_or_default();
                *line = format!("start_with_windows = {enabled}{comment}");
                replaced = true;
                break;
            }
        }
    }

    if !replaced {
        let entry = format!("start_with_windows = {enabled}");
        match first_section {
            Some(index) => lines.insert(index, entry),
            None => lines.push(entry),
        }
    }

    let mut out = lines.join(line_ending);
    if existing.ends_with('\n') || !out.is_empty() {
        out.push_str(line_ending);
    }
    out
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
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let _ = sender.send(event);
    })?;
    let directory = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("config path has no parent"))?;
    watcher.watch(directory, RecursiveMode::NonRecursive)?;

    std::thread::Builder::new()
        .name("ferric-config".to_owned())
        .spawn(move || {
            let _watcher = watcher;
            while let Ok(event) = receiver.recv() {
                if !relevant_event(&event, &path, &app) {
                    continue;
                }

                let mut deadline = Instant::now() + RELOAD_DEBOUNCE;
                loop {
                    let timeout = deadline.saturating_duration_since(Instant::now());
                    match receiver.recv_timeout(timeout) {
                        Ok(event) if relevant_event(&event, &path, &app) => {
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

fn relevant_event(event: &notify::Result<notify::Event>, path: &Path, app: &AppHandle) -> bool {
    match event {
        Ok(event) => {
            matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
            ) && event
                .paths
                .iter()
                .any(|changed| matches_config_path(changed, path))
        }
        Err(error) => {
            tracing::warn!(%error, "config file watcher error");
            report_error(app, format!("Could not watch config.toml: {error}"));
            false
        }
    }
}

fn matches_config_path(changed: &Path, config_path: &Path) -> bool {
    #[cfg(windows)]
    {
        changed
            .file_name()
            .zip(config_path.file_name())
            .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
    }
    #[cfg(not(windows))]
    {
        changed.file_name() == config_path.file_name()
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
    let Some(previous) = app.state::<ConfigManager>().replace(config.clone()) else {
        return;
    };
    if let Err(error) = app.emit(
        CONFIG_EVENT,
        ConfigPayload {
            grain: config.grain,
            errors: Vec::new(),
        },
    ) {
        tracing::warn!(%error, "failed to emit config");
    }
    if previous.hotkeys != config.hotkeys {
        crate::hotkeys::apply(app, &config.hotkeys);
    }
    if previous.start_with_windows != config.start_with_windows {
        crate::autostart::apply(app, config.start_with_windows);
    }
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
                .map(|span| line_number_at(contents, span.start))
                .unwrap_or(1),
        })
}

fn line_number_at(contents: &str, offset: usize) -> usize {
    let clamped = offset.min(contents.len());
    contents.as_bytes()[..clamped]
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1
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

trait IdleTarget {
    fn is_visible(&self) -> bool;
    fn show(&self) -> bool;
    fn hide(&self) -> bool;
    fn set_opacity(&self, opacity: f64) -> bool;
}

struct WindowIdleTarget<'a>(&'a AppHandle);

impl IdleTarget for WindowIdleTarget<'_> {
    fn is_visible(&self) -> bool {
        self.0
            .get_webview_window(MAIN_WINDOW)
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false)
    }

    fn show(&self) -> bool {
        let Some(window) = self.0.get_webview_window(MAIN_WINDOW) else {
            return false;
        };
        if let Err(error) = window.show() {
            tracing::warn!(%error, "failed to show idle window");
            return false;
        }
        crate::tray::sync_visibility(self.0, true);
        true
    }

    fn hide(&self) -> bool {
        let Some(window) = self.0.get_webview_window(MAIN_WINDOW) else {
            return false;
        };
        if let Err(error) = window.hide() {
            tracing::warn!(%error, "failed to hide idle window");
            return false;
        }
        crate::tray::sync_visibility(self.0, false);
        true
    }

    fn set_opacity(&self, opacity: f64) -> bool {
        let Some(window) = self.0.get_webview_window(MAIN_WINDOW) else {
            return false;
        };
        set_window_opacity(&window, opacity)
    }
}

fn start_idle_watcher(app: AppHandle, store: Store) {
    let mut player = store.subscribe();
    let mut config = app.state::<ConfigManager>().subscribe();
    tauri::async_runtime::spawn(async move {
        let target = WindowIdleTarget(&app);
        run_idle_watcher(&target, &mut player, &mut config).await;
    });
}

async fn run_idle_watcher(
    target: &impl IdleTarget,
    player: &mut watch::Receiver<ferric_core::model::PlayerState>,
    config: &mut watch::Receiver<Config>,
) {
    let mut idle_since = None;
    let mut effect = IdleEffect::None;

    loop {
        let playback = player.borrow_and_update().playback;
        if playback == Playback::Playing {
            idle_since = None;
            restore_from_idle(target, &mut effect);
            tokio::select! {
                changed = player.changed() => if changed.is_err() { return },
                changed = config.changed() => if changed.is_err() { return },
            }
            continue;
        }

        let since = *idle_since.get_or_insert_with(tokio::time::Instant::now);
        let deadline = since + IDLE_AFTER;
        if tokio::time::Instant::now() >= deadline {
            let current_config = config.borrow_and_update().clone();
            apply_idle(target, &current_config, &mut effect);
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

fn apply_idle(target: &impl IdleTarget, config: &Config, effect: &mut IdleEffect) {
    if config.hide_when_idle {
        if *effect == IdleEffect::Faded {
            target.set_opacity(1.0);
        }
        if *effect != IdleEffect::Hidden && target.is_visible() && target.hide() {
            *effect = IdleEffect::Hidden;
        }
    } else {
        if *effect == IdleEffect::Hidden && !target.is_visible() && !target.show() {
            return;
        }
        if target.set_opacity(config.idle_opacity) {
            *effect = IdleEffect::Faded;
        }
    }
}

fn restore_from_idle(target: &impl IdleTarget, effect: &mut IdleEffect) {
    match *effect {
        IdleEffect::Faded => {
            target.set_opacity(1.0);
        }
        IdleEffect::Hidden => {
            if !target.is_visible() {
                target.show();
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
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use ferric_core::{model::Playback, store::Store};
    use tokio::sync::watch;

    use super::{
        config_error_message, parse, run_idle_watcher, update_start_with_windows_in_toml, Config,
        Hotkeys, IdleTarget, DEFAULT_CONFIG_TOML, IDLE_AFTER,
    };

    #[derive(Clone)]
    struct FakeWindow {
        state: Arc<Mutex<FakeWindowState>>,
    }

    struct FakeWindowState {
        visible: bool,
        opacity: f64,
    }

    impl FakeWindow {
        fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(FakeWindowState {
                    visible: true,
                    opacity: 1.0,
                })),
            }
        }

        fn visible(&self) -> bool {
            self.state.lock().expect("window state lock").visible
        }

        fn opacity(&self) -> f64 {
            self.state.lock().expect("window state lock").opacity
        }
    }

    impl IdleTarget for FakeWindow {
        fn is_visible(&self) -> bool {
            self.visible()
        }

        fn show(&self) -> bool {
            self.state.lock().expect("window state lock").visible = true;
            true
        }

        fn hide(&self) -> bool {
            self.state.lock().expect("window state lock").visible = false;
            true
        }

        fn set_opacity(&self, opacity: f64) -> bool {
            self.state.lock().expect("window state lock").opacity = opacity;
            true
        }
    }

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
        assert_eq!(
            parse(DEFAULT_CONFIG_TOML).expect("default config template should parse"),
            Config::default()
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

        let negative = parse("grain = -0.3\nidle_opacity = 2.5\n").expect("valid bounds");
        assert_eq!(negative.grain, 0.0);
        assert_eq!(negative.idle_opacity, 1.0);

        let nan_values = parse("grain = nan\nidle_opacity = nan\n").expect("valid floats");
        assert_eq!(nan_values.grain, 0.2);
        assert_eq!(nan_values.idle_opacity, 0.4);
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

        let line_start_error =
            parse("grain = 0.2\r\n\r\n= broken\r\n").expect_err("invalid TOML at line start");
        assert_eq!(line_start_error.line, 3);
    }

    #[test]
    fn updating_start_with_windows_preserves_comments() {
        let updated = update_start_with_windows_in_toml(DEFAULT_CONFIG_TOML, true);
        assert!(updated.contains("# Ferric preferences."));
        assert!(updated.contains("# grain + dust overlay opacity, 0 to 0.5"));
        let parsed = parse(&updated).expect("updated TOML parses");
        assert!(parsed.start_with_windows);
        assert_eq!(parsed.grain, 0.2);
    }

    #[tokio::test(start_paused = true)]
    async fn idle_watcher_fades_hides_and_restores_on_playback() {
        let store = Store::new();
        let (config_tx, mut config_rx) = watch::channel(Config::default());
        let mut player_rx = store.subscribe();
        let window = FakeWindow::new();
        let watcher_window = window.clone();

        let task = tokio::spawn(async move {
            run_idle_watcher(&watcher_window, &mut player_rx, &mut config_rx).await;
        });

        tokio::task::yield_now().await;
        tokio::time::advance(IDLE_AFTER - Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(window.opacity(), 1.0);
        assert!(window.visible());

        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(window.opacity(), 0.4);
        assert!(window.visible());

        config_tx.send_replace(Config {
            hide_when_idle: true,
            ..Config::default()
        });
        tokio::task::yield_now().await;
        assert_eq!(window.opacity(), 1.0);
        assert!(!window.visible());

        store.update(|state| state.playback = Playback::Playing);
        tokio::task::yield_now().await;
        assert!(window.visible());
        assert_eq!(window.opacity(), 1.0);

        store.update(|state| state.playback = Playback::Paused);
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(10)).await;
        store.update(|state| state.playback = Playback::Playing);
        tokio::task::yield_now().await;
        tokio::time::advance(IDLE_AFTER).await;
        tokio::task::yield_now().await;
        assert!(window.visible());
        assert_eq!(window.opacity(), 1.0);

        task.abort();
    }
}
