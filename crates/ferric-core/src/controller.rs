use crate::command::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handled {
    Done,
    Unsupported,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ControlError {
    #[error("Spotify is not running")]
    NotRunning,
    #[error("Spotify Premium is required for this")]
    PremiumRequired,
    #[error("not signed in")]
    NotSignedIn,
    #[error("rate limited; retry after {0}s")]
    RateLimited(u64),
    #[error("{0}")]
    Other(String),
}

#[async_trait::async_trait]
pub trait Controller: Send + Sync {
    fn name(&self) -> &'static str;

    async fn execute(&self, cmd: &Command) -> Result<Handled, ControlError>;
}
