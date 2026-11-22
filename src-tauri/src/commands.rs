use ferric_core::{command::Command, model::PlayerState, store::Store};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::watch;

use crate::router::Router;

#[tauri::command]
pub async fn get_state(store: State<'_, Store>) -> Result<PlayerState, String> {
    Ok(store.get())
}

#[tauri::command]
pub async fn open_spotify(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url("spotify:", None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn send_command(router: State<'_, Router>, cmd: Command) -> Result<(), String> {
    router.execute(&cmd).await
}

#[tauri::command]
pub async fn set_queue_visible(
    queue_visible: State<'_, watch::Sender<bool>>,
    visible: bool,
) -> Result<(), String> {
    queue_visible.send_replace(visible);
    Ok(())
}
