use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn apply(app: &AppHandle, enabled: bool) {
    let manager = app.autolaunch();
    if manager.is_enabled().ok() != Some(enabled) {
        let result = if enabled {
            manager.enable()
        } else {
            manager.disable()
        };

        if let Err(error) = result {
            tracing::warn!(%error, enabled, "failed to update start-with-Windows setting");
            crate::config::report_error(
                app,
                format!("Could not update Start with Windows: {error}"),
            );
        }
    }

    let actual = manager.is_enabled().unwrap_or(enabled);
    crate::tray::sync_autostart(app, actual);
}
