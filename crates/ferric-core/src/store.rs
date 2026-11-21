use std::sync::Arc;

use tokio::sync::watch;

use crate::model::PlayerState;

#[derive(Clone)]
pub struct Store {
    sender: Arc<watch::Sender<PlayerState>>,
}

impl Store {
    pub fn new() -> Self {
        let (sender, _) = watch::channel(PlayerState::default());
        Self {
            sender: Arc::new(sender),
        }
    }

    pub fn get(&self) -> PlayerState {
        self.sender.borrow().clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut PlayerState)) {
        self.sender.send_if_modified(|state| {
            let before = state.clone();
            f(state);
            *state != before
        });
    }

    pub fn subscribe(&self) -> watch::Receiver<PlayerState> {
        self.sender.subscribe()
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Store;
    use crate::model::Playback;

    #[tokio::test]
    async fn update_notifies_when_state_changes() {
        let store = Store::new();
        let mut receiver = store.subscribe();

        store.update(|state| state.playback = Playback::Playing);

        receiver
            .changed()
            .await
            .expect("the store sender should remain alive");
        assert_eq!(receiver.borrow().playback, Playback::Playing);
    }

    #[test]
    fn update_does_not_notify_when_state_is_unchanged() {
        let store = Store::new();
        let receiver = store.subscribe();

        store.update(|state| state.playback = Playback::Stopped);

        assert!(!receiver.has_changed().expect("sender should remain alive"));
    }

    #[test]
    fn clones_share_state() {
        let store = Store::new();
        let clone = store.clone();
        clone.update(|state| state.spotify_running = true);
        assert!(store.get().spotify_running);
    }
}
