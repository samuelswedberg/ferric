use std::sync::Arc;

use ferric_core::store::Store;

pub use crate::error::AuthError;

pub trait TokenStore: Send + Sync {
    fn load(&self) -> Option<StoredAuth>;
    fn save(&self, auth: &StoredAuth) -> Result<(), AuthError>;
    fn clear(&self) -> Result<(), AuthError>;
}

pub struct KeyringTokenStore;

impl TokenStore for KeyringTokenStore {
    fn load(&self) -> Option<StoredAuth> {
        None
    }

    fn save(&self, _auth: &StoredAuth) -> Result<(), AuthError> {
        Err(AuthError::Unimplemented)
    }

    fn clear(&self) -> Result<(), AuthError> {
        Err(AuthError::Unimplemented)
    }
}

pub struct StoredAuth {
    pub client_id: String,
    pub refresh_token: Option<String>,
}

pub struct Authenticator {
    _store: Store,
    _tokens: Arc<dyn TokenStore>,
}

impl Authenticator {
    pub fn new(store: Store, tokens: Arc<dyn TokenStore>) -> Self {
        Self {
            _store: store,
            _tokens: tokens,
        }
    }

    pub async fn set_client_id(&self, _id: String) -> Result<(), AuthError> {
        Err(AuthError::Unimplemented)
    }

    pub async fn sign_in(&self) -> Result<(), AuthError> {
        Err(AuthError::Unimplemented)
    }

    pub async fn sign_out(&self) -> Result<(), AuthError> {
        Err(AuthError::Unimplemented)
    }

    pub async fn restore(&self) -> Result<(), AuthError> {
        Err(AuthError::Unimplemented)
    }

    pub fn client(&self) -> Option<rspotify::AuthCodePkceSpotify> {
        None
    }
}
