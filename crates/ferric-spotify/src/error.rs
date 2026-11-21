#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Spotify authentication is not implemented")]
    Unimplemented,
}
