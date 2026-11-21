use std::sync::Arc;

use async_trait::async_trait;
use ferric_core::{
    command::Command,
    controller::{ControlError, Controller, Handled},
    store::Store,
};

use crate::auth::Authenticator;

pub struct SpotifyController {
    _store: Store,
    _auth: Arc<Authenticator>,
}

impl SpotifyController {
    pub fn new(store: Store, auth: Arc<Authenticator>) -> Self {
        Self {
            _store: store,
            _auth: auth,
        }
    }
}

#[async_trait]
impl Controller for SpotifyController {
    fn name(&self) -> &'static str {
        "spotify"
    }

    async fn execute(&self, _cmd: &Command) -> Result<Handled, ControlError> {
        Ok(Handled::Unsupported)
    }
}
