#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[cfg(windows)]
    #[error("Windows media controls failed: {0}")]
    Windows(#[from] windows::core::Error),
    #[cfg(windows)]
    #[error("thumbnail is too large to cache")]
    ThumbnailTooLarge,
    #[error("thumbnail cache failed: {0}")]
    Io(#[from] std::io::Error),
}
