mod app;
mod auth;
mod autostart;
mod commands;
mod config;
mod hotkeys;
mod router;
mod theming;
mod tray;
mod window;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::SIZE,
                )
                .skip_initial_state("main")
                .build(),
        )
        .manage(ferric_core::store::Store::new())
        .setup(|app| {
            app::setup(app)?;
            window::setup(app.handle())?;
            tray::build(app.handle())?;
            config::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth::auth_set_client_id,
            auth::auth_sign_in,
            auth::auth_sign_out,
            commands::get_state,
            commands::open_spotify,
            commands::send_command,
            commands::set_queue_visible,
            config::get_config,
            window::set_mode_command
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Ferric");
}
