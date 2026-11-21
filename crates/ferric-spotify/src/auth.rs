use std::{
    collections::{HashMap, HashSet},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    time::Duration,
};

use ferric_core::{model::Auth, store::Store};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::SubscriptionLevel,
    AuthCodePkceSpotify, CallbackError, Config, Credentials, OAuth, Token, TokenCallback,
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

pub struct Authenticator {
    store: Store,
    tokens: Arc<dyn TokenStore>,
    client: RwLock<Option<AuthCodePkceSpotify>>,
    busy: AtomicBool,
}

impl Authenticator {
    pub fn new(store: Store, tokens: Arc<dyn TokenStore>) -> Self {
        Self {
            store,
            tokens,
            client: RwLock::new(None),
            busy: AtomicBool::new(false),
        }
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
            self.set_auth(if matches!(error, AuthError::NoClientId) {
                Auth::NoClientId
            } else {
                Auth::SignedOut
            });
        }
        result
    }

    async fn sign_in_inner(&self) -> Result<(), AuthError> {
        let stored = self.tokens.load().ok_or(AuthError::NoClientId)?;
        validate_client_id(&stored.client_id)?;
        self.set_auth(Auth::SigningIn);

        let listener = TcpListener::bind(CALLBACK_ADDR).map_err(|_| AuthError::PortInUse)?;
        let server = Server::from_listener(listener, None)
            .map_err(|error| AuthError::Worker(error.to_string()))?;
        let mut client = build_client(&stored.client_id, Arc::clone(&self.tokens));
        let expected_state = client.oauth.state.clone();
        let authorize_url = client.get_authorize_url(None).map_err(spotify_error)?;

        tokio::task::spawn_blocking(move || open::that(authorize_url))
            .await
            .map_err(|error| AuthError::Worker(error.to_string()))?
            .map_err(|error| AuthError::Browser(error.to_string()))?;
        let code = tokio::task::spawn_blocking(move || {
            receive_callback(server, &expected_state, CALLBACK_TIMEOUT)
        })
        .await
        .map_err(|error| AuthError::Worker(error.to_string()))??;

        client.request_token(&code).await.map_err(spotify_error)?;
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

        let profile = client.current_user().await.map_err(spotify_error)?;
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

        let client = build_client(&stored.client_id, Arc::clone(&self.tokens));
        *client
            .token
            .lock()
            .await
            .expect("Spotify token lock poisoned") = Some(Token {
            refresh_token: Some(refresh_token),
            ..Token::default()
        });
        if let Err(error) = client.refresh_token().await {
            if is_invalid_grant(&error) {
                self.tokens.save(&StoredAuth {
                    client_id: stored.client_id,
                    refresh_token: None,
                })?;
                self.set_auth(Auth::SignedOut);
                return Ok(());
            }
            let error = spotify_error(error);
            self.set_error(&error);
            return Err(error);
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
                let error = spotify_error(error);
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

fn build_client(client_id: &str, tokens: Arc<dyn TokenStore>) -> AuthCodePkceSpotify {
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

fn spotify_error(error: rspotify::ClientError) -> AuthError {
    AuthError::Spotify(error.to_string())
}

fn is_invalid_grant(error: &rspotify::ClientError) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    // rspotify 0.16 discards OAuth error response bodies. A 400 from the refresh
    // endpoint with Ferric's fixed, validated request can only be a rejected grant.
    message.contains("invalid_grant") || message.ends_with("status code 400")
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENT_ID: &str = "0123456789abcdef0123456789abcdef";

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
}
