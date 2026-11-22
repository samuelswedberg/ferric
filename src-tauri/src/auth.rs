use std::sync::Arc;

use ferric_core::store::Store;
use ferric_spotify::auth::{Authenticator, KeyringTokenStore};
use tauri::State;
use tokio::sync::watch;

pub type AuthState = Arc<Authenticator>;

pub fn setup(store: Store, queue_visible: watch::Receiver<bool>) -> AuthState {
    let auth = Arc::new(Authenticator::new(
        store.clone(),
        Arc::new(KeyringTokenStore),
    ));

    let restoring_auth = Arc::clone(&auth);
    tauri::async_runtime::spawn(async move {
        if let Err(error) = restoring_auth.restore().await {
            tracing::warn!(%error, "could not restore Spotify sign-in");
        }
    });

    let polling_auth = Arc::clone(&auth);
    tauri::async_runtime::spawn(async move {
        ferric_spotify::poller::run(store, polling_auth, queue_visible).await;
    });

    auth
}

#[tauri::command]
pub async fn auth_set_client_id(
    auth: State<'_, AuthState>,
    client_id: String,
) -> Result<(), String> {
    auth.set_client_id(client_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn auth_sign_in(auth: State<'_, AuthState>) -> Result<(), String> {
    auth.sign_in().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn auth_sign_out(auth: State<'_, AuthState>) -> Result<(), String> {
    auth.sign_out().await.map_err(|error| error.to_string())
}
