use std::path::Path;

use ferric_core::model::Theme;

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("theme extraction is not implemented")]
    Unimplemented,
}

pub fn extract(
    _art_bytes: &[u8],
    _album_key: &str,
    _cache_dir: &Path,
) -> Result<Theme, ThemeError> {
    Err(ThemeError::Unimplemented)
}
