use std::sync::Arc;

use ferric_core::store::Store;
use tokio::sync::watch;

use crate::auth::Authenticator;

pub async fn run(_store: Store, _auth: Arc<Authenticator>, _visible: watch::Receiver<bool>) {}
