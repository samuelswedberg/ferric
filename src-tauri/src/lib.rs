mod app;
mod commands;
mod router;
mod theming;
mod tray;
mod window;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::get_state])
        .run(tauri::generate_context!())
        .expect("failed to run Ferric");
}
