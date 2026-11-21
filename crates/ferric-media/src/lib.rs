mod controller;
mod error;

use std::path::PathBuf;

use ferric_core::store::Store;

pub use controller::MediaController;
pub use error::MediaError;

pub async fn run(_store: Store, _art_cache_dir: PathBuf) -> Result<(), MediaError> {
    Err(MediaError::Unimplemented)
}
