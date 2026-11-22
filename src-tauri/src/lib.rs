mod app;
mod commands;
mod router;
mod theming;
mod tray;
mod window;

pub fn run() {
    tauri::Builder::default()
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
            window::setup(app.handle())?;
            tray::build(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            window::set_mode_command
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Ferric");
}
