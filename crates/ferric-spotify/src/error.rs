#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Spotify client ID must be exactly 32 hexadecimal characters")]
    InvalidClientId,
    #[error("set a Spotify client ID before signing in")]
    NoClientId,
    #[error("port 8898 is already in use; close the other application and try again")]
    PortInUse,
    #[error("Spotify sign-in timed out")]
    Timeout,
    #[error("Spotify returned an invalid OAuth state")]
    InvalidState,
    #[error("Spotify denied authorization: {0}")]
    AuthorizationDenied(String),
    #[error("the Spotify callback was invalid: {0}")]
    InvalidCallback(String),
    #[error("could not open the system browser: {0}")]
    Browser(String),
    #[error("credential storage failed: {0}")]
    CredentialStore(String),
    #[error("Spotify authentication failed: {0}")]
    Spotify(String),
    #[error("an authentication operation is already in progress")]
    Busy,
    #[error("authentication worker failed: {0}")]
    Worker(String),
}
