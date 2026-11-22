use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use ferric_core::{model::PlayerState, store::Store};
use ferric_media::MediaController;
use tauri::{Emitter, Manager};
use tokio::{sync::watch, time::Instant};
use tracing_subscriber::{
    fmt::MakeWriter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

use crate::router::Router;

const STATE_EVENT: &str = "ferric://state";
const EMIT_INTERVAL: Duration = Duration::from_nanos(33_333_334);
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

pub fn setup(app: &mut tauri::App) -> anyhow::Result<()> {
    init_logging(app)?;

    let art_cache_dir = app.path().app_cache_dir()?.join("art");
    fs::create_dir_all(&art_cache_dir)?;

    let store = app.state::<Store>().inner().clone();
    let (queue_visible, _) = watch::channel(false);
    app.manage(queue_visible);
    spawn_media_watcher(store.clone(), art_cache_dir.clone())?;

    let emitter_store = store.clone();
    let emitter_app = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        run_emitter(emitter_store, move |state| {
            emitter_app
                .emit(STATE_EVENT, state)
                .map_err(|error| error.to_string())
        })
        .await;
    });

    tauri::async_runtime::spawn(crate::theming::run(store.clone(), art_cache_dir));

    // T10: spotify auth + poller here
    let queue_visible_rx = app.state::<watch::Sender<bool>>().subscribe();
    let auth = crate::auth::setup(store.clone(), queue_visible_rx);
    let spotify = ferric_spotify::SpotifyController::new(store, Arc::clone(&auth));
    app.manage(auth);
    app.manage(Router::new(vec![
        Arc::new(MediaController::new()),
        Arc::new(spotify),
    ]));

    Ok(())
}

fn spawn_media_watcher(store: Store, art_cache_dir: PathBuf) -> io::Result<()> {
    std::thread::Builder::new()
        .name("ferric-media".to_owned())
        .spawn(move || {
            #[cfg(windows)]
            let _apartment = match WindowsRuntimeApartment::initialize() {
                Ok(apartment) => apartment,
                Err(error) => {
                    tracing::error!(%error, "failed to initialize media watcher WinRT apartment");
                    return;
                }
            };

            if let Err(error) =
                tauri::async_runtime::block_on(ferric_media::run(store, art_cache_dir))
            {
                tracing::error!(%error, "media watcher stopped");
            }
        })?;
    Ok(())
}

#[cfg(windows)]
struct WindowsRuntimeApartment;

#[cfg(windows)]
impl WindowsRuntimeApartment {
    fn initialize() -> windows::core::Result<Self> {
        use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

        // SAFETY: this is the first WinRT call on a newly created dedicated
        // thread, and the matching RoUninitialize runs on that same thread.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for WindowsRuntimeApartment {
    fn drop(&mut self) {
        use windows::Win32::System::WinRT::RoUninitialize;

        // SAFETY: paired with the successful RoInitialize on this thread.
        unsafe { RoUninitialize() };
    }
}

async fn run_emitter<F>(store: Store, mut emit: F)
where
    F: FnMut(&PlayerState) -> Result<(), String>,
{
    let mut receiver = store.subscribe();
    let mut next_emit = Instant::now();

    while receiver.changed().await.is_ok() {
        let mut pending = receiver.borrow_and_update().clone();

        loop {
            let now = Instant::now();
            if now >= next_emit {
                emit_state(&mut emit, &pending);
                next_emit = now + EMIT_INTERVAL;
                break;
            }

            tokio::select! {
                biased;
                _ = tokio::time::sleep_until(next_emit) => {
                    emit_state(&mut emit, &pending);
                    next_emit = Instant::now() + EMIT_INTERVAL;
                    break;
                }
                changed = receiver.changed() => {
                    if changed.is_err() {
                        tokio::time::sleep_until(next_emit).await;
                        emit_state(&mut emit, &pending);
                        return;
                    }
                    pending = receiver.borrow_and_update().clone();
                }
            }
        }
    }
}

fn emit_state<F>(emit: &mut F, state: &PlayerState)
where
    F: FnMut(&PlayerState) -> Result<(), String>,
{
    if let Err(error) = emit(state) {
        tracing::warn!(%error, "failed to emit player state");
    }
}

fn init_logging(app: &tauri::App) -> anyhow::Result<()> {
    let log_dir = app.path().app_log_dir()?;
    fs::create_dir_all(&log_dir)?;
    let file_writer = RollingFileWriter::new(log_dir.join("ferric.log"), MAX_LOG_BYTES);
    let filter = EnvFilter::try_from_env("FERRIC_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file_writer),
        )
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize logging: {error}"))?;
    Ok(())
}

#[derive(Clone)]
struct RollingFileWriter {
    path: PathBuf,
    rotated_path: PathBuf,
    max_bytes: u64,
    lock: Arc<Mutex<()>>,
}

impl RollingFileWriter {
    fn new(path: PathBuf, max_bytes: u64) -> Self {
        let rotated_path = path.with_extension("log.1");
        Self {
            path,
            rotated_path,
            max_bytes,
            lock: Arc::new(Mutex::new(())),
        }
    }
}

impl<'a> MakeWriter<'a> for RollingFileWriter {
    type Writer = RollingWriter;

    fn make_writer(&'a self) -> Self::Writer {
        RollingWriter(self.clone())
    }
}

struct RollingWriter(RollingFileWriter);

impl Write for RollingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let _guard = self
            .0
            .lock
            .lock()
            .map_err(|_| io::Error::other("log writer lock poisoned"))?;
        rotate_if_needed(
            &self.0.path,
            &self.0.rotated_path,
            self.0.max_bytes,
            buffer.len() as u64,
        )?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.0.path)?
            .write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn rotate_if_needed(
    path: &Path,
    rotated_path: &Path,
    max_bytes: u64,
    incoming: u64,
) -> io::Result<()> {
    let current = fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if current > 0 && current.saturating_add(incoming) > max_bytes {
        match fs::remove_file(rotated_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::rename(path, rotated_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use ferric_core::{model::Playback, store::Store};

    use super::{run_emitter, EMIT_INTERVAL};

    #[tokio::test(start_paused = true)]
    async fn emitter_throttles_to_thirty_per_second_and_keeps_the_trailing_state() {
        let store = Store::new();
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let task_store = store.clone();
        let task_emitted = Arc::clone(&emitted);
        let task = tokio::spawn(async move {
            run_emitter(task_store, move |state| {
                task_emitted
                    .lock()
                    .expect("emitted state lock")
                    .push(state.clone());
                Ok(())
            })
            .await;
        });
        tokio::task::yield_now().await;

        store.update(|state| state.spotify_running = true);
        tokio::task::yield_now().await;
        assert_eq!(emitted.lock().expect("emitted state lock").len(), 1);

        store.update(|state| state.playback = Playback::Paused);
        store.update(|state| state.playback = Playback::Playing);
        tokio::task::yield_now().await;
        assert_eq!(emitted.lock().expect("emitted state lock").len(), 1);

        tokio::time::advance(EMIT_INTERVAL - Duration::from_millis(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(emitted.lock().expect("emitted state lock").len(), 1);

        tokio::time::advance(Duration::from_millis(2)).await;
        tokio::task::yield_now().await;
        let states = emitted.lock().expect("emitted state lock");
        assert_eq!(states.len(), 2);
        assert_eq!(states[1].playback, Playback::Playing);

        task.abort();
    }
}
