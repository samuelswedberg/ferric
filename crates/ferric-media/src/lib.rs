mod controller;
mod error;
#[cfg(any(windows, test))]
mod mapping;

#[cfg(windows)]
mod windows;

use std::path::PathBuf;

use ferric_core::store::Store;

pub use controller::MediaController;
pub use error::MediaError;

pub async fn run(store: Store, art_cache_dir: PathBuf) -> Result<(), MediaError> {
    #[cfg(windows)]
    {
        windows::run(store, art_cache_dir).await
    }

    #[cfg(not(windows))]
    {
        let _ = (store, art_cache_dir);
        Ok(())
    }
}
