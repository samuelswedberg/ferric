use std::{
    collections::VecDeque,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    time::Duration,
};

use ferric_core::{
    model::{Art, Theme},
    store::Store,
};

const CACHE_CAPACITY: usize = 32;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(5);

type ThemeFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

trait ThemeEngine: Send + Sync {
    fn load<'a>(&'a self, art: &'a Art) -> ThemeFuture<'a, Result<Vec<u8>, String>>;

    fn extract(
        &self,
        bytes: Vec<u8>,
        album_key: String,
        cache_dir: PathBuf,
    ) -> ThemeFuture<'static, Result<Theme, String>>;
}

struct FerricThemeEngine {
    client: reqwest::Client,
}

impl FerricThemeEngine {
    fn new() -> Result<Self, reqwest::Error> {
        reqwest::Client::builder()
            .timeout(DOWNLOAD_TIMEOUT)
            .build()
            .map(|client| Self { client })
    }
}

impl ThemeEngine for FerricThemeEngine {
    fn load<'a>(&'a self, art: &'a Art) -> ThemeFuture<'a, Result<Vec<u8>, String>> {
        Box::pin(async move {
            match art {
                Art::File { path } => tokio::fs::read(path)
                    .await
                    .map_err(|error| format!("could not read album art: {error}")),
                Art::Url { url } => self
                    .client
                    .get(url)
                    .send()
                    .await
                    .and_then(reqwest::Response::error_for_status)
                    .map_err(|error| format!("could not download album art: {error}"))?
                    .bytes()
                    .await
                    .map(|bytes| bytes.to_vec())
                    .map_err(|error| format!("could not read album art response: {error}")),
            }
        })
    }

    fn extract(
        &self,
        bytes: Vec<u8>,
        album_key: String,
        cache_dir: PathBuf,
    ) -> ThemeFuture<'static, Result<Theme, String>> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                ferric_theme::extract(&bytes, &album_key, &cache_dir)
                    .map_err(|error| error.to_string())
            })
            .await
            .map_err(|error| format!("theme extraction task failed: {error}"))?
        })
    }
}

struct ThemeCache {
    entries: VecDeque<(String, Theme)>,
}

impl ThemeCache {
    fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(CACHE_CAPACITY),
        }
    }

    fn get(&mut self, album_key: &str) -> Option<Theme> {
        let index = self.entries.iter().position(|(key, _)| key == album_key)?;
        let entry = self.entries.remove(index)?;
        let theme = entry.1.clone();
        self.entries.push_front(entry);
        Some(theme)
    }

    fn insert(&mut self, theme: Theme) {
        if let Some(index) = self
            .entries
            .iter()
            .position(|(key, _)| key == &theme.album_key)
        {
            self.entries.remove(index);
        }
        self.entries.push_front((theme.album_key.clone(), theme));
        self.entries.truncate(CACHE_CAPACITY);
    }
}

pub async fn run(store: Store, cache_dir: PathBuf) {
    let engine = match FerricThemeEngine::new() {
        Ok(engine) => engine,
        Err(error) => {
            tracing::error!(%error, "failed to create album-art HTTP client");
            return;
        }
    };
    run_with_engine(store, cache_dir, &engine).await;
}

async fn run_with_engine<E: ThemeEngine>(store: Store, cache_dir: PathBuf, engine: &E) {
    let mut receiver = store.subscribe();
    let mut last_art: Option<Option<Art>> = None;
    let mut cache = ThemeCache::new();

    loop {
        let art = receiver
            .borrow_and_update()
            .track
            .as_ref()
            .and_then(|track| track.art.clone());

        if last_art.as_ref() != Some(&art) {
            last_art = Some(art.clone());
            update_theme(&store, &cache_dir, engine, &mut cache, art).await;
        }

        if receiver.changed().await.is_err() {
            return;
        }
    }
}

async fn update_theme<E: ThemeEngine>(
    store: &Store,
    cache_dir: &Path,
    engine: &E,
    cache: &mut ThemeCache,
    art: Option<Art>,
) {
    let Some(art) = art else {
        store.update(|state| state.theme = None);
        return;
    };

    let bytes = match engine.load(&art).await {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(%error, "failed to load album art for theming");
            return;
        }
    };
    let album_key = album_key(&bytes);
    let theme = if let Some(theme) = cache.get(&album_key) {
        theme
    } else {
        match engine
            .extract(bytes, album_key, cache_dir.to_path_buf())
            .await
        {
            Ok(theme) => {
                cache.insert(theme.clone());
                theme
            }
            Err(error) => {
                tracing::warn!(%error, "failed to extract album theme");
                return;
            }
        }
    };

    let current_art = store
        .get()
        .track
        .as_ref()
        .and_then(|track| track.art.clone());
    if current_art.as_ref() == Some(&art) {
        store.update(|state| state.theme = Some(theme));
    } else {
        tracing::debug!("discarded stale album theme");
    }
}

fn album_key(bytes: &[u8]) -> String {
    // Stable FNV-1a is sufficient for a small, process-local LRU and produces a
    // filesystem-safe key for ferric-theme's persistent artifacts.
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    format!("art-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use ferric_core::{
        model::{Art, Rgb, Theme, Track},
        store::Store,
    };
    use tokio::sync::Notify;

    use super::{album_key, run_with_engine, ThemeCache, ThemeEngine, ThemeFuture, CACHE_CAPACITY};

    struct FakeEngine {
        started: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
        release: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
    }

    impl FakeEngine {
        fn new(keys: &[String]) -> Self {
            let notifications = || {
                keys.iter()
                    .map(|key| (key.clone(), Arc::new(Notify::new())))
                    .collect()
            };
            Self {
                started: Arc::new(Mutex::new(notifications())),
                release: Arc::new(Mutex::new(notifications())),
            }
        }

        fn notification(map: &Mutex<HashMap<String, Arc<Notify>>>, key: &str) -> Arc<Notify> {
            Arc::clone(
                map.lock()
                    .expect("notification map lock")
                    .get(key)
                    .expect("known key"),
            )
        }
    }

    impl ThemeEngine for FakeEngine {
        fn load<'a>(&'a self, art: &'a Art) -> ThemeFuture<'a, Result<Vec<u8>, String>> {
            Box::pin(async move {
                match art {
                    Art::File { path } => Ok(path.as_bytes().to_vec()),
                    Art::Url { url } => Ok(url.as_bytes().to_vec()),
                }
            })
        }

        fn extract(
            &self,
            _bytes: Vec<u8>,
            album_key: String,
            _cache_dir: PathBuf,
        ) -> ThemeFuture<'static, Result<Theme, String>> {
            let started = Self::notification(&self.started, &album_key);
            let release = Self::notification(&self.release, &album_key);
            Box::pin(async move {
                started.notify_one();
                release.notified().await;
                Ok(theme(&album_key))
            })
        }
    }

    fn theme(key: &str) -> Theme {
        Theme {
            album_key: key.to_owned(),
            accent: Rgb { r: 1, g: 2, b: 3 },
            on_accent: Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            sleeve: None,
            record: None,
        }
    }

    fn set_art(store: &Store, path: &str) {
        store.update(|state| {
            state.track = Some(Track {
                spotify_id: None,
                title: path.to_owned(),
                artists: vec!["Artist".to_owned()],
                album: None,
                album_id: None,
                duration_ms: None,
                art: Some(Art::File {
                    path: path.to_owned(),
                }),
            });
        });
    }

    #[tokio::test]
    async fn stale_extraction_never_overwrites_the_new_track_theme() {
        let first_key = album_key(b"first");
        let second_key = album_key(b"second");
        let engine = Arc::new(FakeEngine::new(&[first_key.clone(), second_key.clone()]));
        let store = Store::new();
        set_art(&store, "first");

        let task_store = store.clone();
        let task_engine = Arc::clone(&engine);
        let task = tokio::spawn(async move {
            run_with_engine(task_store, PathBuf::from("cache"), task_engine.as_ref()).await;
        });

        FakeEngine::notification(&engine.started, &first_key)
            .notified()
            .await;
        set_art(&store, "second");
        FakeEngine::notification(&engine.release, &first_key).notify_one();
        FakeEngine::notification(&engine.started, &second_key)
            .notified()
            .await;

        assert_eq!(
            store.get().theme,
            None,
            "the stale first theme was committed"
        );

        let mut receiver = store.subscribe();
        FakeEngine::notification(&engine.release, &second_key).notify_one();
        tokio::time::timeout(Duration::from_secs(1), async {
            while store.get().theme.as_ref().map(|value| &value.album_key) != Some(&second_key) {
                receiver.changed().await.expect("store remains open");
            }
        })
        .await
        .expect("second theme should be committed");

        task.abort();
    }

    #[test]
    fn lru_keeps_only_the_most_recent_thirty_two_themes() {
        let mut cache = ThemeCache::new();
        for index in 0..=CACHE_CAPACITY {
            cache.insert(theme(&format!("album-{index}")));
        }

        assert!(cache.get("album-0").is_none());
        assert!(cache.get("album-1").is_some());
        assert!(cache.get(&format!("album-{CACHE_CAPACITY}")).is_some());
    }
}
