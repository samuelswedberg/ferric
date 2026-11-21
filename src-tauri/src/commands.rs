//! Bootstrap IPC commands. T08 replaces this with the wired application state.

use ferric_core::model::PlayerState;

#[tauri::command]
pub async fn get_state() -> PlayerState {
    PlayerState::default()
}
