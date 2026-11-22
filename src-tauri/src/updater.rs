use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const UPDATE_EVENT: &str = "ferric://update-available";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateAvailable {
    version: String,
}

pub(crate) fn configured() -> bool {
    update_checks_enabled(option_env!("FERRIC_UPDATER_PUBKEY"))
}

pub(crate) fn setup(app: &AppHandle) {
    if !configured() {
        tracing::info!("updater is disabled because FERRIC_UPDATER_PUBKEY is not configured");
        return;
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            check_and_emit(&app).await;
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

async fn check_and_emit(app: &AppHandle) {
    let result = async {
        let updater = app.updater().map_err(|error| error.to_string())?;
        updater.check().await.map_err(|error| error.to_string())
    }
    .await;

    match result {
        Ok(Some(update)) => {
            if let Err(error) = app.emit(
                UPDATE_EVENT,
                UpdateAvailable {
                    version: update.version.to_string(),
                },
            ) {
                tracing::warn!(%error, "failed to show the available update");
            }
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "update check failed"),
    }
}

#[tauri::command]
pub(crate) async fn install_update(app: AppHandle) -> Result<(), String> {
    if !configured() {
        return Err("updates are not configured in this build".to_owned());
    }

    let updater = app.updater().map_err(|error| error.to_string())?;
    let Some(update) = updater.check().await.map_err(|error| error.to_string())? else {
        return Err("the update is no longer available".to_owned());
    };

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| error.to_string())?;
    app.restart();
}

fn update_checks_enabled(pubkey: Option<&str>) -> bool {
    pubkey.is_some_and(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::update_checks_enabled;

    #[test]
    fn updater_stays_inert_without_a_non_empty_public_key() {
        assert!(!update_checks_enabled(None));
        assert!(!update_checks_enabled(Some("  \n")));
        assert!(update_checks_enabled(Some("public-key")));
    }
}
