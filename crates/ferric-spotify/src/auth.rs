use std::{
    collections::{HashMap, HashSet},
    net::{SocketAddr, TcpListener},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Duration,
};

use ferric_core::{model::Auth, store::Store};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    http::HttpError,
    model::SubscriptionLevel,
    AuthCodePkceSpotify, CallbackError, ClientError, Config, Credentials, OAuth, Token,
    TokenCallback, DEFAULT_API_BASE_URL, DEFAULT_AUTH_BASE_URL,
};
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Response, Server, StatusCode};
use url::Url;

pub use crate::error::AuthError;

const REDIRECT_URI: &str = "http://127.0.0.1:8898/callback";
const CALLBACK_ADDR: &str = "127.0.0.1:8898";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(3 * 60);
const CALLBACK_SUCCESS: &str = "You're signed in to Ferric. You can close this tab.";
const SCOPES: [&str; 5] = [
    "user-read-playback-state",
    "user-read-currently-playing",
    "user-modify-playback-state",
    "user-library-read",
    "user-library-modify",
];

pub trait TokenStore: Send + Sync {
    fn load(&self) -> Option<StoredAuth>;
    fn save(&self, auth: &StoredAuth) -> Result<(), AuthError>;
    fn clear(&self) -> Result<(), AuthError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredAuth {
    pub client_id: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyringTokenStore;

impl KeyringTokenStore {
    fn entry() -> Result<keyring::Entry, AuthError> {
        keyring::Entry::new("ferric", "spotify")
            .map_err(|error| AuthError::CredentialStore(error.to_string()))
    }
}

impl TokenStore for KeyringTokenStore {
    fn load(&self) -> Option<StoredAuth> {
        let entry = match Self::entry() {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(error = %error, "could not open Spotify credential storage");
                return None;
            }
        };
        let password = match entry.get_password() {
            Ok(password) => password,
            Err(keyring::Error::NoEntry) => return None,
            Err(error) => {
                tracing::warn!(error = %error, "could not load Spotify credentials");
                return None;
            }
        };

        match serde_json::from_str(&password) {
            Ok(auth) => Some(auth),
            Err(error) => {
                tracing::warn!(error = %error, "stored Spotify credentials were invalid");
                None
            }
        }
    }

    fn save(&self, auth: &StoredAuth) -> Result<(), AuthError> {
        let value = serde_json::to_string(auth)
            .map_err(|error| AuthError::CredentialStore(error.to_string()))?;
        Self::entry()?
            .set_password(&value)
            .map_err(|error| AuthError::CredentialStore(error.to_string()))
    }

    fn clear(&self) -> Result<(), AuthError> {
        let entry = Self::entry()?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(AuthError::CredentialStore(error.to_string())),
        }
    }
}

#[derive(Debug, Default)]
pub struct MemoryTokenStore {
    auth: Mutex<Option<StoredAuth>>,
}

impl MemoryTokenStore {
    pub fn new(auth: Option<StoredAuth>) -> Self {
        Self {
            auth: Mutex::new(auth),
        }
    }
}

impl TokenStore for MemoryTokenStore {
    fn load(&self) -> Option<StoredAuth> {
        self.auth
            .lock()
            .expect("memory token store poisoned")
            .clone()
    }

    fn save(&self, auth: &StoredAuth) -> Result<(), AuthError> {
        *self.auth.lock().expect("memory token store poisoned") = Some(auth.clone());
        Ok(())
    }

    fn clear(&self) -> Result<(), AuthError> {
        *self.auth.lock().expect("memory token store poisoned") = None;
        Ok(())
    }
}

type BrowserOpener = Arc<dyn Fn(String, SocketAddr) -> Result<(), String> + Send + Sync>;

struct AuthRuntime {
    callback_addr: String,
    callback_timeout: Duration,
    auth_base_url: String,
    api_base_url: String,
    open_browser: BrowserOpener,
}

impl Default for AuthRuntime {
    fn default() -> Self {
        Self {
            callback_addr: CALLBACK_ADDR.to_owned(),
            callback_timeout: CALLBACK_TIMEOUT,
            auth_base_url: DEFAULT_AUTH_BASE_URL.to_owned(),
            api_base_url: DEFAULT_API_BASE_URL.to_owned(),
            open_browser: Arc::new(|url, _| open::that(url).map_err(|error| error.to_string())),
        }
    }
}

pub struct Authenticator {
    store: Store,
    tokens: Arc<dyn TokenStore>,
    client: RwLock<Option<AuthCodePkceSpotify>>,
    busy: AtomicBool,
    runtime: AuthRuntime,
    /// When the user last set the volume (unix ms); see `volume_recently_set`.
    volume_set_at_ms: AtomicU64,
}

impl Authenticator {
    pub fn new(store: Store, tokens: Arc<dyn TokenStore>) -> Self {
        Self::with_runtime(store, tokens, AuthRuntime::default())
    }

    fn with_runtime(store: Store, tokens: Arc<dyn TokenStore>, runtime: AuthRuntime) -> Self {
        Self {
            store,
            tokens,
            client: RwLock::new(None),
            busy: AtomicBool::new(false),
            runtime,
            volume_set_at_ms: AtomicU64::new(0),
        }
    }

    /// Records that the user just set the volume (called before and after the request).
    pub(crate) fn note_volume_set(&self) {
        self.volume_set_at_ms
            .store(now_unix_ms(), Ordering::Relaxed);
    }

    /// True for 2 s after the user set the volume: a poll then may still report the old
    /// value and must not move the knob back.
    pub(crate) fn volume_recently_set(&self) -> bool {
        const HOLD_MS: u64 = 2_000;
        now_unix_ms().saturating_sub(self.volume_set_at_ms.load(Ordering::Relaxed)) < HOLD_MS
    }

    pub async fn set_client_id(&self, id: String) -> Result<(), AuthError> {
        validate_client_id(&id)?;
        self.tokens.save(&StoredAuth {
            client_id: id,
            refresh_token: None,
        })?;
        *self.client.write().expect("auth client lock poisoned") = None;
        self.set_auth(Auth::SignedOut);
        Ok(())
    }

    pub async fn sign_in(&self) -> Result<(), AuthError> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(AuthError::Busy);
        }
        let result = self.sign_in_inner().await;
        self.busy.store(false, Ordering::Release);
        if let Err(error) = &result {
            self.set_auth(auth_state_for_sign_in_error(error));
        }
        result
    }

    async fn sign_in_inner(&self) -> Result<(), AuthError> {
        let stored = self.tokens.load().ok_or(AuthError::NoClientId)?;
        validate_client_id(&stored.client_id)?;
        self.set_auth(Auth::SigningIn);

        let listener =
            TcpListener::bind(&self.runtime.callback_addr).map_err(|_| AuthError::PortInUse)?;
        let local_addr = listener
            .local_addr()
            .map_err(|error| AuthError::Worker(error.to_string()))?;
        let server = Server::from_listener(listener, None)
            .map_err(|error| AuthError::Worker(error.to_string()))?;
        let mut client = build_client(
            &stored.client_id,
            Arc::clone(&self.tokens),
            &self.runtime.auth_base_url,
            &self.runtime.api_base_url,
        );
        let expected_state = client.oauth.state.clone();
        let authorize_url = client.get_authorize_url(None).map_err(spotify_error)?;

        let open_browser = Arc::clone(&self.runtime.open_browser);
        tokio::task::spawn_blocking(move || open_browser(authorize_url, local_addr))
            .await
            .map_err(|error| AuthError::Worker(error.to_string()))?
            .map_err(AuthError::Browser)?;
        let callback_timeout = self.runtime.callback_timeout;
        let code = tokio::task::spawn_blocking(move || {
            receive_callback(server, &expected_state, callback_timeout)
        })
        .await
        .map_err(|error| AuthError::Worker(error.to_string()))??;

        if let Err(error) = client.request_token(&code).await {
            return Err(parse_spotify_error(error).await.auth_error);
        }
        let refresh_token = client
            .token
            .lock()
            .await
            .expect("Spotify token lock poisoned")
            .as_ref()
            .and_then(|token| token.refresh_token.clone())
            .ok_or_else(|| AuthError::Spotify("Spotify returned no refresh token".into()))?;
        self.tokens.save(&StoredAuth {
            client_id: stored.client_id,
            refresh_token: Some(refresh_token),
        })?;

        let profile = match client.current_user().await {
            Ok(profile) => profile,
            Err(error) => return Err(parse_spotify_error(error).await.auth_error),
        };
        let display_name = profile
            .display_name
            .unwrap_or_else(|| profile.id.to_string());
        #[allow(deprecated)]
        let premium = profile.product == Some(SubscriptionLevel::Premium);
        *self.client.write().expect("auth client lock poisoned") = Some(client);
        self.set_auth(Auth::SignedIn {
            display_name,
            premium,
        });
        Ok(())
    }

    pub async fn sign_out(&self) -> Result<(), AuthError> {
        let stored = self.tokens.load().ok_or(AuthError::NoClientId)?;
        self.tokens.save(&StoredAuth {
            client_id: stored.client_id,
            refresh_token: None,
        })?;
        *self.client.write().expect("auth client lock poisoned") = None;
        self.set_auth(Auth::SignedOut);
        Ok(())
    }

    pub async fn restore(&self) -> Result<(), AuthError> {
        let Some(stored) = self.tokens.load() else {
            self.set_auth(Auth::NoClientId);
            return Ok(());
        };
        if let Err(error) = validate_client_id(&stored.client_id) {
            self.set_error(&error);
            return Err(error);
        }
        let Some(refresh_token) = stored.refresh_token.clone() else {
            self.set_auth(Auth::SignedOut);
            return Ok(());
        };

        let client = build_client(
            &stored.client_id,
            Arc::clone(&self.tokens),
            &self.runtime.auth_base_url,
            &self.runtime.api_base_url,
        );
        *client
            .token
            .lock()
            .await
            .expect("Spotify token lock poisoned") = Some(Token {
            refresh_token: Some(refresh_token),
            ..Token::default()
        });
        if let Err(error) = client.refresh_token().await {
            let parsed = parse_spotify_error(error).await;
            if parsed.is_invalid_grant {
                if let Err(save_error) = self.tokens.save(&StoredAuth {
                    client_id: stored.client_id,
                    refresh_token: None,
                }) {
                    self.set_error(&save_error);
                    return Err(save_error);
                }
                self.set_auth(Auth::SignedOut);
                return Ok(());
            }
            self.set_error(&parsed.auth_error);
            return Err(parsed.auth_error);
        }

        // Spotify may omit a replacement refresh token. Retain the stored one in that case.
        let mut token = client
            .token
            .lock()
            .await
            .expect("Spotify token lock poisoned");
        if let Some(token) = token.as_mut() {
            if token.refresh_token.is_none() {
                token.refresh_token = stored.refresh_token;
            }
        }
        drop(token);

        let profile = match client.current_user().await {
            Ok(profile) => profile,
            Err(error) => {
                let error = parse_spotify_error(error).await.auth_error;
                self.set_error(&error);
                return Err(error);
            }
        };
        let display_name = profile
            .display_name
            .unwrap_or_else(|| profile.id.to_string());
        #[allow(deprecated)]
        let premium = profile.product == Some(SubscriptionLevel::Premium);
        *self.client.write().expect("auth client lock poisoned") = Some(client);
        self.set_auth(Auth::SignedIn {
            display_name,
            premium,
        });
        Ok(())
    }

    pub fn client(&self) -> Option<AuthCodePkceSpotify> {
        self.client
            .read()
            .expect("auth client lock poisoned")
            .clone()
    }

    fn set_auth(&self, auth: Auth) {
        self.store.update(|state| state.auth = auth);
    }

    fn set_error(&self, error: &AuthError) {
        self.set_auth(Auth::Error {
            message: error.to_string(),
        });
    }
}

fn build_client(
    client_id: &str,
    tokens: Arc<dyn TokenStore>,
    auth_base_url: &str,
    api_base_url: &str,
) -> AuthCodePkceSpotify {
    let callback_client_id = client_id.to_owned();
    let callback = TokenCallback(Box::new(move |token| {
        if let Some(refresh_token) = token.refresh_token {
            tokens
                .save(&StoredAuth {
                    client_id: callback_client_id.clone(),
                    refresh_token: Some(refresh_token),
                })
                .map_err(|error| CallbackError::CustomizedError(error.to_string()))?;
        }
        Ok(())
    }));
    let config = Config {
        auth_base_url: auth_base_url.to_owned(),
        api_base_url: api_base_url.to_owned(),
        token_cached: false,
        token_refreshing: true,
        token_callback_fn: Arc::new(Some(callback)),
        ..Config::default()
    };
    let oauth = OAuth {
        redirect_uri: REDIRECT_URI.into(),
        scopes: SCOPES
            .into_iter()
            .map(str::to_owned)
            .collect::<HashSet<_>>(),
        ..OAuth::default()
    };
    AuthCodePkceSpotify::with_config(Credentials::new_pkce(client_id), oauth, config)
}

fn receive_callback(
    server: Server,
    expected_state: &str,
    timeout: Duration,
) -> Result<String, AuthError> {
    let request = server
        .recv_timeout(timeout)
        .map_err(|error| AuthError::InvalidCallback(error.to_string()))?
        .ok_or(AuthError::Timeout)?;
    let result = parse_callback(request.url(), expected_state);
    let (status, body) = match &result {
        Ok(_) => (StatusCode(200), CALLBACK_SUCCESS),
        Err(_) => (StatusCode(400), "Spotify sign-in could not be completed."),
    };
    let mut response =
        Response::from_string(format!("<!doctype html><html><body>{body}</body></html>"))
            .with_status_code(status);
    if let Ok(header) = Header::from_bytes("Content-Type", "text/html; charset=utf-8") {
        response.add_header(header);
    }
    request
        .respond(response)
        .map_err(|error| AuthError::InvalidCallback(error.to_string()))?;
    result
}

fn parse_callback(request_target: &str, expected_state: &str) -> Result<String, AuthError> {
    let url = Url::parse(&format!("http://127.0.0.1{request_target}"))
        .map_err(|error| AuthError::InvalidCallback(error.to_string()))?;
    if url.path() != "/callback" {
        return Err(AuthError::InvalidCallback(
            "unexpected callback path".into(),
        ));
    }
    let params: HashMap<_, _> = url.query_pairs().into_owned().collect();
    if params.get("state").map(String::as_str) != Some(expected_state) {
        return Err(AuthError::InvalidState);
    }
    if let Some(error) = params.get("error") {
        return Err(AuthError::AuthorizationDenied(error.clone()));
    }
    params
        .get("code")
        .filter(|code| !code.is_empty())
        .cloned()
        .ok_or_else(|| AuthError::InvalidCallback("missing authorization code".into()))
}

fn validate_client_id(id: &str) -> Result<(), AuthError> {
    if id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(AuthError::InvalidClientId)
    }
}

fn auth_state_for_sign_in_error(error: &AuthError) -> Auth {
    match error {
        AuthError::NoClientId => Auth::NoClientId,
        AuthError::AuthorizationDenied(code) if code.eq_ignore_ascii_case("access_denied") => {
            Auth::SignedOut
        }
        other => Auth::Error {
            message: other.to_string(),
        },
    }
}

fn spotify_error(error: ClientError) -> AuthError {
    AuthError::Spotify(error.to_string())
}

#[derive(Deserialize)]
struct OAuthErrorBody {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct WebApiErrorEnvelope {
    error: WebApiErrorBody,
}

#[derive(Deserialize)]
struct WebApiErrorBody {
    message: String,
}

struct ParsedSpotifyError {
    is_invalid_grant: bool,
    auth_error: AuthError,
}

async fn parse_spotify_error(error: ClientError) -> ParsedSpotifyError {
    match error {
        ClientError::Http(http_error) => match *http_error {
            HttpError::StatusCode(response) => {
                let status = response.status();
                let body = response.text().await.ok();
                if let Some(oauth) = body
                    .as_deref()
                    .and_then(|text| serde_json::from_str::<OAuthErrorBody>(text).ok())
                {
                    let is_invalid_grant =
                        status.as_u16() == 400 && oauth.error.eq_ignore_ascii_case("invalid_grant");
                    let detail = match oauth
                        .error_description
                        .as_deref()
                        .map(str::trim)
                        .filter(|description| !description.is_empty())
                    {
                        Some(description) => format!(
                            "http error: status code {status} ({}: {description})",
                            oauth.error
                        ),
                        None => format!("http error: status code {status} ({})", oauth.error),
                    };
                    return ParsedSpotifyError {
                        is_invalid_grant,
                        auth_error: AuthError::Spotify(detail),
                    };
                }
                if let Some(web_api) = body
                    .as_deref()
                    .and_then(|text| serde_json::from_str::<WebApiErrorEnvelope>(text).ok())
                {
                    let message = web_api.error.message.trim();
                    let detail = if message.is_empty() {
                        format!("http error: status code {status}")
                    } else {
                        format!("http error: status code {status} ({message})")
                    };
                    return ParsedSpotifyError {
                        is_invalid_grant: false,
                        auth_error: AuthError::Spotify(detail),
                    };
                }
                ParsedSpotifyError {
                    is_invalid_grant: false,
                    auth_error: AuthError::Spotify(format!("http error: status code {status}")),
                }
            }
            other => ParsedSpotifyError {
                is_invalid_grant: false,
                auth_error: AuthError::Spotify(format!("http error: {other}")),
            },
        },
        other => ParsedSpotifyError {
            is_invalid_grant: false,
            auth_error: AuthError::Spotify(other.to_string()),
        },
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpStream,
        thread,
    };

    use super::*;

    const CLIENT_ID: &str = "0123456789abcdef0123456789abcdef";

    fn send_http_request(addr: SocketAddr, target: &str) {
        let mut stream = TcpStream::connect(addr).unwrap();
        write!(
            stream,
            "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);
    }

    fn extract_state(authorize_url: &str) -> String {
        let parsed = Url::parse(authorize_url).unwrap();
        parsed
            .query_pairs()
            .find_map(|(key, value)| (key == "state").then(|| value.into_owned()))
            .expect("authorize URL missing state")
    }

    fn spawn_single_response_server(
        status: u16,
        content_type: &'static str,
        body: &'static str,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = Server::from_listener(listener, None).unwrap();
        let handle = thread::spawn(move || {
            let request = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .expect("expected request to mock server");
            let mut response = Response::from_string(body).with_status_code(StatusCode(status));
            if let Ok(header) = Header::from_bytes("Content-Type", content_type) {
                response.add_header(header);
            }
            request.respond(response).unwrap();
        });
        (format!("http://{addr}/"), handle)
    }

    fn test_runtime(open_browser: BrowserOpener) -> AuthRuntime {
        AuthRuntime {
            callback_addr: "127.0.0.1:0".into(),
            callback_timeout: Duration::from_secs(2),
            auth_base_url: DEFAULT_AUTH_BASE_URL.into(),
            api_base_url: DEFAULT_API_BASE_URL.into(),
            open_browser,
        }
    }

    #[test]
    fn client_id_validation_is_exact() {
        assert!(validate_client_id(CLIENT_ID).is_ok());
        assert!(validate_client_id("ABCDEF0123456789ABCDEF0123456789").is_ok());
        assert!(matches!(
            validate_client_id("short"),
            Err(AuthError::InvalidClientId)
        ));
        assert!(matches!(
            validate_client_id("g123456789abcdef0123456789abcdef"),
            Err(AuthError::InvalidClientId)
        ));
    }

    #[test]
    fn callback_requires_matching_state() {
        assert_eq!(
            parse_callback("/callback?code=abc&state=expected", "expected").unwrap(),
            "abc"
        );
        assert!(matches!(
            parse_callback("/callback?code=abc&state=wrong", "expected"),
            Err(AuthError::InvalidState)
        ));
    }

    #[test]
    fn callback_listener_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let server = Server::from_listener(listener, None).unwrap();
        assert!(matches!(
            receive_callback(server, "state", Duration::from_millis(1)),
            Err(AuthError::Timeout)
        ));
    }

    #[tokio::test]
    async fn set_client_id_clears_token_and_sign_out_keeps_id() {
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: Some("old-refresh-token".into()),
        })));
        let auth = Authenticator::new(store.clone(), tokens.clone());

        auth.set_client_id(CLIENT_ID.to_uppercase()).await.unwrap();
        assert_eq!(store.get().auth, Auth::SignedOut);
        assert_eq!(tokens.load().unwrap().refresh_token, None);
        auth.sign_out().await.unwrap();
        assert_eq!(tokens.load().unwrap().client_id, CLIENT_ID.to_uppercase());
        assert_eq!(store.get().auth, Auth::SignedOut);
    }

    #[tokio::test]
    async fn restore_without_client_id_sets_no_client_id() {
        let store = Store::new();
        let auth = Authenticator::new(store.clone(), Arc::new(MemoryTokenStore::default()));
        auth.restore().await.unwrap();
        assert_eq!(store.get().auth, Auth::NoClientId);
    }

    #[tokio::test]
    async fn restore_without_refresh_token_sets_signed_out() {
        let store = Store::new();
        let tokens = MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: None,
        }));
        let auth = Authenticator::new(store.clone(), Arc::new(tokens));
        auth.restore().await.unwrap();
        assert_eq!(store.get().auth, Auth::SignedOut);
        assert!(auth.client().is_none());
    }

    #[tokio::test]
    async fn sign_in_without_client_id_sets_no_client_id() {
        let store = Store::new();
        let auth = Authenticator::with_runtime(
            store.clone(),
            Arc::new(MemoryTokenStore::default()),
            test_runtime(Arc::new(|_, _| panic!("browser should not open"))),
        );

        let err = auth.sign_in().await.unwrap_err();
        assert!(matches!(err, AuthError::NoClientId));
        assert_eq!(store.get().auth, Auth::NoClientId);
    }

    #[tokio::test]
    async fn sign_in_port_in_use_sets_error_state() {
        let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied_addr = occupied.local_addr().unwrap().to_string();
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: None,
        })));
        let mut runtime = test_runtime(Arc::new(|_, _| panic!("browser should not open")));
        runtime.callback_addr = occupied_addr;
        let auth = Authenticator::with_runtime(store.clone(), tokens, runtime);

        let err = auth.sign_in().await.unwrap_err();
        assert!(matches!(err, AuthError::PortInUse));
        assert_eq!(
            store.get().auth,
            Auth::Error {
                message: AuthError::PortInUse.to_string(),
            }
        );
        drop(occupied);
    }

    #[tokio::test]
    async fn sign_in_timeout_sets_error_state() {
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: None,
        })));
        let mut runtime = test_runtime(Arc::new(|_, _| Ok(())));
        runtime.callback_timeout = Duration::from_millis(20);
        let auth = Authenticator::with_runtime(store.clone(), tokens, runtime);

        let err = auth.sign_in().await.unwrap_err();
        assert!(matches!(err, AuthError::Timeout));
        assert_eq!(
            store.get().auth,
            Auth::Error {
                message: AuthError::Timeout.to_string(),
            }
        );
    }

    #[tokio::test]
    async fn sign_in_invalid_state_sets_error_state() {
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: None,
        })));
        let runtime = test_runtime(Arc::new(|_, addr| {
            thread::spawn(move || {
                send_http_request(addr, "/callback?code=abc&state=wrong");
            });
            Ok(())
        }));
        let auth = Authenticator::with_runtime(store.clone(), tokens, runtime);

        let err = auth.sign_in().await.unwrap_err();
        assert!(matches!(err, AuthError::InvalidState));
        assert_eq!(
            store.get().auth,
            Auth::Error {
                message: AuthError::InvalidState.to_string(),
            }
        );
    }

    #[tokio::test]
    async fn sign_in_user_cancelled_sets_signed_out() {
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: None,
        })));
        let runtime = test_runtime(Arc::new(|authorize_url, addr| {
            let state = extract_state(&authorize_url);
            thread::spawn(move || {
                send_http_request(
                    addr,
                    &format!("/callback?error=access_denied&state={state}"),
                );
            });
            Ok(())
        }));
        let auth = Authenticator::with_runtime(store.clone(), tokens, runtime);

        let err = auth.sign_in().await.unwrap_err();
        assert!(matches!(
            err,
            AuthError::AuthorizationDenied(ref code) if code == "access_denied"
        ));
        assert_eq!(store.get().auth, Auth::SignedOut);
    }

    #[tokio::test]
    async fn sign_in_spotify_callback_error_sets_error_state() {
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: None,
        })));
        let runtime = test_runtime(Arc::new(|authorize_url, addr| {
            let state = extract_state(&authorize_url);
            thread::spawn(move || {
                send_http_request(addr, &format!("/callback?error=server_error&state={state}"));
            });
            Ok(())
        }));
        let auth = Authenticator::with_runtime(store.clone(), tokens, runtime);

        let err = auth.sign_in().await.unwrap_err();
        assert!(matches!(
            err,
            AuthError::AuthorizationDenied(ref code) if code == "server_error"
        ));
        assert_eq!(
            store.get().auth,
            Auth::Error {
                message: err.to_string(),
            }
        );
    }

    #[tokio::test]
    async fn sign_in_spotify_token_error_sets_error_state() {
        let (auth_base_url, handle) = spawn_single_response_server(
            400,
            "application/json",
            r#"{"error":"invalid_client","error_description":"Invalid client"}"#,
        );
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: None,
        })));
        let mut runtime = test_runtime(Arc::new(|authorize_url, addr| {
            let state = extract_state(&authorize_url);
            thread::spawn(move || {
                send_http_request(addr, &format!("/callback?code=test-code&state={state}"));
            });
            Ok(())
        }));
        runtime.auth_base_url = auth_base_url;
        let auth = Authenticator::with_runtime(store.clone(), tokens, runtime);

        let err = auth.sign_in().await.unwrap_err();
        handle.join().unwrap();
        assert!(matches!(err, AuthError::Spotify(_)));
        assert_eq!(
            store.get().auth,
            Auth::Error {
                message: err.to_string(),
            }
        );
        match store.get().auth {
            Auth::Error { message } => {
                assert!(message.contains("invalid_client"), "got: {message}");
                assert!(message.contains("Invalid client"), "got: {message}");
            }
            other => panic!("expected Auth::Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn sign_in_browser_error_sets_error_state() {
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: None,
        })));
        let runtime = test_runtime(Arc::new(|_, _| Err("browser unavailable".into())));
        let auth = Authenticator::with_runtime(store.clone(), tokens, runtime);

        let err = auth.sign_in().await.unwrap_err();
        assert!(matches!(err, AuthError::Browser(_)));
        assert_eq!(
            store.get().auth,
            Auth::Error {
                message: err.to_string(),
            }
        );
    }

    #[tokio::test]
    async fn restore_invalid_grant_400_clears_refresh_token_and_signs_out() {
        let (auth_base_url, handle) = spawn_single_response_server(
            400,
            "application/json",
            r#"{"error":"invalid_grant","error_description":"Refresh token revoked"}"#,
        );
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: Some("revoked-refresh-token".into()),
        })));
        let mut runtime = test_runtime(Arc::new(|_, _| panic!("browser should not open")));
        runtime.auth_base_url = auth_base_url;
        let auth = Authenticator::with_runtime(store.clone(), tokens.clone(), runtime);

        auth.restore().await.unwrap();
        handle.join().unwrap();
        assert_eq!(store.get().auth, Auth::SignedOut);
        assert_eq!(
            tokens.load(),
            Some(StoredAuth {
                client_id: CLIENT_ID.into(),
                refresh_token: None,
            })
        );
    }

    #[tokio::test]
    async fn restore_other_400_sets_error_and_keeps_refresh_token() {
        let (auth_base_url, handle) = spawn_single_response_server(
            400,
            "application/json",
            r#"{"error":"invalid_client","error_description":"Invalid client"}"#,
        );
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: Some("kept-refresh-token".into()),
        })));
        let mut runtime = test_runtime(Arc::new(|_, _| panic!("browser should not open")));
        runtime.auth_base_url = auth_base_url;
        let auth = Authenticator::with_runtime(store.clone(), tokens.clone(), runtime);

        let err = auth.restore().await.unwrap_err();
        handle.join().unwrap();
        assert!(matches!(err, AuthError::Spotify(_)));
        assert_eq!(
            store.get().auth,
            Auth::Error {
                message: err.to_string(),
            }
        );
        match store.get().auth {
            Auth::Error { message } => {
                assert!(message.contains("invalid_client"), "got: {message}");
            }
            other => panic!("expected Auth::Error, got {other:?}"),
        }
        assert_eq!(
            tokens.load().unwrap().refresh_token.as_deref(),
            Some("kept-refresh-token")
        );
    }

    #[tokio::test]
    async fn restore_non_json_400_sets_error_and_keeps_refresh_token() {
        let (auth_base_url, handle) =
            spawn_single_response_server(400, "text/plain", "Bad Request");
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: Some("kept-refresh-token".into()),
        })));
        let mut runtime = test_runtime(Arc::new(|_, _| panic!("browser should not open")));
        runtime.auth_base_url = auth_base_url;
        let auth = Authenticator::with_runtime(store.clone(), tokens.clone(), runtime);

        let err = auth.restore().await.unwrap_err();
        handle.join().unwrap();
        assert!(matches!(err, AuthError::Spotify(_)));
        assert_eq!(
            store.get().auth,
            Auth::Error {
                message: err.to_string(),
            }
        );
        assert_eq!(
            tokens.load().unwrap().refresh_token.as_deref(),
            Some("kept-refresh-token")
        );
    }

    #[tokio::test]
    async fn restore_500_with_invalid_grant_body_is_not_treated_as_invalid_grant() {
        let (auth_base_url, handle) = spawn_single_response_server(
            500,
            "application/json",
            r#"{"error":"invalid_grant","error_description":"transient"}"#,
        );
        let store = Store::new();
        let tokens = Arc::new(MemoryTokenStore::new(Some(StoredAuth {
            client_id: CLIENT_ID.into(),
            refresh_token: Some("kept-refresh-token".into()),
        })));
        let mut runtime = test_runtime(Arc::new(|_, _| panic!("browser should not open")));
        runtime.auth_base_url = auth_base_url;
        let auth = Authenticator::with_runtime(store.clone(), tokens.clone(), runtime);

        let err = auth.restore().await.unwrap_err();
        handle.join().unwrap();
        assert!(matches!(err, AuthError::Spotify(_)));
        assert_eq!(
            store.get().auth,
            Auth::Error {
                message: err.to_string(),
            }
        );
        assert_eq!(
            tokens.load().unwrap().refresh_token.as_deref(),
            Some("kept-refresh-token")
        );
    }
}
