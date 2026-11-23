mod app;
mod auth;
mod autostart;
mod commands;
mod config;
mod hotkeys;
mod router;
mod shortcut;
mod theming;
mod tray;
mod updater;
mod window;

pub fn run() {
    let mut builder = tauri::Builder::default();
    if updater::configured() {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
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
            updater::setup(app.handle());
            window::setup(app.handle())?;
            tray::build(app.handle())?;
            config::setup(app)?;
            shortcut::setup();
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
            updater::install_update,
            window::set_mode_command
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Ferric");
}
