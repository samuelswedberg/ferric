use std::sync::Arc;

use ferric_core::{model::Auth, store::Store};
use ferric_spotify::auth::{Authenticator, KeyringTokenStore, TokenStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let supplied_client_id = std::env::args().nth(1);
    let store = Store::new();
    let tokens = Arc::new(KeyringTokenStore);
    let authenticator = Authenticator::new(store.clone(), tokens.clone());

    let stored = tokens.load();
    match (supplied_client_id, stored) {
        (Some(client_id), Some(stored))
            if stored.client_id == client_id && stored.refresh_token.is_some() =>
        {
            authenticator.restore().await?;
        }
        (Some(client_id), _) => {
            authenticator.set_client_id(client_id).await?;
            authenticator.sign_in().await?;
        }
        (None, Some(stored)) if stored.refresh_token.is_some() => {
            authenticator.restore().await?;
        }
        (None, _) => {
            return Err(
                "usage: cargo run -p ferric-spotify --example sign_in -- <client_id>".into(),
            );
        }
    }

    match store.get().auth {
        Auth::SignedIn {
            display_name,
            premium,
        } => println!("Signed in as {display_name} (Premium: {premium})"),
        other => return Err(format!("sign-in ended in unexpected state: {other:?}").into()),
    }
    Ok(())
}
