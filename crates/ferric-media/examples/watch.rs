use std::sync::Arc;

use ferric_core::{command::Command, controller::Controller, store::Store};
use ferric_media::{MediaController, MediaError};

#[tokio::main]
async fn main() -> Result<(), MediaError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    let store = Store::new();
    let mut states = store.subscribe();
    let controller: Arc<dyn Controller> = Arc::new(MediaController::new());
    let command = std::env::args()
        .nth(1)
        .and_then(|flag| match flag.as_str() {
            "--toggle" => Some(Command::PlayPause),
            "--next" => Some(Command::Next),
            "--prev" => Some(Command::Previous),
            _ => None,
        });
    if let Some(command) = command {
        controller
            .execute(&command)
            .await
            .map_err(|error| std::io::Error::other(error.to_string()))?;
    }

    tokio::spawn(async move {
        while states.changed().await.is_ok() {
            tracing::info!(state = ?states.borrow_and_update(), "media state changed");
        }
    });

    let cache_dir = std::env::temp_dir().join("ferric-media-art");
    ferric_media::run(store, cache_dir).await
}
