# Contracts

These are the types and APIs every task builds against. **T00** creates them exactly as written here, in `crates/ferric-core`. After T00 they change only through a dedicated `feat(core):` PR that updates this file too.

All IPC types derive `Serialize, Deserialize, ts_rs::TS` with `#[serde(rename_all = "camelCase")]` and `#[ts(export, export_to = "../../../ui/src/lib/bindings/")]`.

## 1. State model: `ferric_core::model`

```rust
pub struct PlayerState {
    pub spotify_running: bool,
    pub track: Option<Track>,
    pub playback: Playback,
    pub position: Option<Position>,
    pub volume_pct: Option<u8>,          // 0..=100, Web API only
    pub liked: Option<bool>,             // Web API only
    pub shuffle: Option<bool>,           // Web API only
    pub repeat: Option<RepeatMode>,      // Web API only
    pub queue: Option<Vec<Track>>,       // None = unknown / signed out; Some(vec![]) = empty
    pub auth: Auth,
    pub theme: Option<Theme>,
    pub mode: Mode,
}
// Default: spotify_running=false, playback=Stopped, auth=NoClientId, mode=Full, the rest None.

pub enum RepeatMode { Off, Context, Track }

pub struct Track {
    pub spotify_id: Option<String>,      // None when only SMTC knows the track
    pub title: String,
    pub artists: Vec<String>,
    pub artist_id: Option<String>,       // Spotify primary artist id; None when only SMTC knows the track
    pub album: Option<String>,
    pub album_id: Option<String>,        // Spotify album id; None when only SMTC knows the track
    pub duration_ms: Option<u64>,
    pub art: Option<Art>,
}

#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Art {
    File { path: String },               // absolute path in the app cache dir
    Url { url: String },                 // i.scdn.co cover URL from the Web API
}

pub enum Playback { Playing, Paused, Stopped }

pub struct Position {
    pub ms: u64,
    pub sampled_at_unix_ms: u64,         // the UI extrapolates while Playing
}

#[serde(tag = "status", rename_all = "camelCase")]
pub enum Auth {
    NoClientId,
    SignedOut,
    SigningIn,
    SignedIn { display_name: String, premium: bool },
    Error { message: String },
}

pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }

pub struct Theme {
    pub album_key: String,               // cache key: spotify album id, or a hash of the art bytes
    pub accent: Rgb,                     // >= 4.5:1 against the plinth #1E1B18
    pub on_accent: Rgb,                  // text/icons on top of the accent color
    pub sleeve: Option<Art>,             // worn sleeve: the cover with ring wear, rubbed corners, crease (Art::File, PNG)
    pub record: Option<Art>,             // dusty record with the worn cover as its label, transparent outside (Art::File, PNG)
}
// The plinth texture, grain overlay and cardboard don't depend on the album: T03 generates
// them once with design/tools/textures.py and commits them as static PNGs in
// ui/src/lib/assets/textures/ (plinth.png, grain.png, card.png).

pub enum Mode { Full, Card, Strip }
```

### Who writes which field

Each field has exactly one writer. Read anything; write only your own fields.

| Field(s) | Writer |
| --- | --- |
| `spotify_running`, `track` (title, artists, album, art from SMTC), `playback`, `position` when signed out | `ferric-media` |
| `track.spotify_id`, `track.artist_id`, `track.album_id`, `track.duration_ms`, `track.art` when the Web API gives a bigger cover, `queue`, `liked`, `shuffle`, `repeat`, `volume_pct`, `position` when signed in | `ferric-spotify` poller. It enriches the track only if the title and first artist match what SMTC reported. |
| `auth` | `ferric-spotify` auth |
| `theme` | `src-tauri` wiring (calls `ferric-theme`) |
| `mode` | `src-tauri` window module |

## 2. Commands: `ferric_core::command`

```rust
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Command {
    PlayPause,
    Next,
    Previous,
    Seek { position_ms: u64 },
    SetVolume { pct: u8 },
    SetLiked { liked: bool },
    SetPlaybackMode { shuffle: bool, repeat: RepeatMode },
    JumpTo { queue_index: u32 },         // 0 = next up; rejected if > 9
}
```

## 3. Controllers: `ferric_core::controller`

```rust
pub enum Handled { Done, Unsupported }   // Unsupported => the router tries the next controller

#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    #[error("Spotify is not running")] NotRunning,
    #[error("Spotify Premium is required for this")] PremiumRequired,
    #[error("not signed in")] NotSignedIn,
    #[error("rate limited; retry after {0}s")] RateLimited(u64),
    #[error("{0}")] Other(String),
}

#[async_trait::async_trait]
pub trait Controller: Send + Sync {
    fn name(&self) -> &'static str;
    async fn execute(&self, cmd: &Command) -> Result<Handled, ControlError>;
}
```

Router order: `MediaController` first (PlayPause/Next/Previous), then `SpotifyController` (everything; also the fallback for the first three).

## 4. Store: `ferric_core::store`

```rust
#[derive(Clone)]
pub struct Store { /* Arc<tokio::sync::watch::Sender<PlayerState>> */ }

impl Store {
    pub fn new() -> Self;
    pub fn get(&self) -> PlayerState;                          // clone of the current state
    pub fn update(&self, f: impl FnOnce(&mut PlayerState));    // notifies only if the state changed (send_if_modified + PartialEq)
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<PlayerState>;
}
```

## 5. Crate entry points (T00 stubs them; the owning task fills them in)

```rust
// ferric-media (T01)
pub async fn run(store: Store, art_cache_dir: PathBuf) -> Result<(), MediaError>; // runs until cancelled
pub struct MediaController;          // impl Controller
impl MediaController { pub fn new() -> Self; }

// ferric-spotify (T02 auth, T07 api)
pub mod auth {
    pub trait TokenStore: Send + Sync { fn load(&self) -> Option<StoredAuth>; fn save(&self, a: &StoredAuth) -> Result<(), AuthError>; fn clear(&self) -> Result<(), AuthError>; }
    pub struct KeyringTokenStore;    // service "ferric", user "spotify"
    pub struct StoredAuth { pub client_id: String, pub refresh_token: Option<String> }
    pub struct Authenticator;        // new(store: Store, tokens: Arc<dyn TokenStore>)
    impl Authenticator {
        pub async fn set_client_id(&self, id: String) -> Result<(), AuthError>;
        pub async fn sign_in(&self) -> Result<(), AuthError>;  // opens browser, waits on 127.0.0.1:8898
        pub async fn sign_out(&self) -> Result<(), AuthError>;
        pub async fn restore(&self) -> Result<(), AuthError>;  // at startup, from the stored refresh token
        pub fn client(&self) -> Option<rspotify::AuthCodePkceSpotify>;
    }
}
pub mod poller { pub async fn run(store: Store, auth: Arc<Authenticator>, visible: watch::Receiver<bool>); }
pub struct SpotifyController;        // impl Controller; new(store: Store, auth: Arc<Authenticator>)

// ferric-theme (T03)
pub fn extract(art_bytes: &[u8], album_key: &str, cache_dir: &Path) -> Result<Theme, ThemeError>; // CPU-bound: call via spawn_blocking
```

## 6. Tauri IPC

Event (Rust → UI):

| Event | Payload | When |
| --- | --- | --- |
| `ferric://state` | `PlayerState` | every store change, throttled to at most 30 per second |

Commands (UI → Rust), all `async`, errors returned as `String`:

| Command | Args | Returns | Owner |
| --- | --- | --- | --- |
| `get_state` | none | `PlayerState` | T08 |
| `send_command` | `cmd: Command` | `()` | T08 |
| `set_mode` | `mode: Mode` | `()` | T05 |
| `set_queue_visible` | `visible: bool` | `()` | T08 |
| `open_spotify` | none | `()` | T08 |
| `auth_set_client_id` | `client_id: String` | `()` | T10 |
| `auth_sign_in` | none | `()` | T10 |
| `auth_sign_out` | none | `()` | T10 |

The UI's typed wrappers live in `ui/src/lib/ipc.ts` (created by T00, with a mock backend for browser dev: `npm run dev` with no Tauri).
