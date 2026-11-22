#![forbid(unsafe_code)]

mod color;
mod noise;
mod render;

use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use ferric_core::model::{Art, Rgb, Theme};
use image::imageops::FilterType;

use crate::noise::seed_from_key;

// Bump when the rendered sleeve/record look changes, so cached art is redrawn.
const CACHE_VERSION: &str = "3";
const SLEEVE_SALT: u64 = 0x534c_4545_5645_0001;
const RECORD_SALT: u64 = 0x5245_434f_5244_0002;

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("could not decode album art: {0}")]
    Decode(#[source] image::ImageError),
    #[error("theme cache I/O failed: {0}")]
    Io(#[from] io::Error),
}

#[doc(hidden)]
pub mod textures {
    pub use crate::render::{
        render_card, render_grain, render_plinth, write_best_rgb_png, write_best_rgba_png,
    };
}

/// Derives a readable accent color, worn sleeve image, and dusty record image from album art.
///
/// This function is CPU-bound and synchronous; callers in async code should use
/// `spawn_blocking`.
pub fn extract(art_bytes: &[u8], album_key: &str, cache_dir: &Path) -> Result<Theme, ThemeError> {
    fs::create_dir_all(cache_dir)?;
    let cache_key = safe_cache_key(album_key);
    let sleeve_path = cache_dir.join(format!("sleeve-{cache_key}.png"));
    let record_path = cache_dir.join(format!("record-{cache_key}.png"));
    let meta_path = cache_dir.join(format!("theme-{cache_key}.meta"));

    if sleeve_path.is_file() && record_path.is_file() {
        if let Some(accent) = read_metadata(&meta_path)? {
            return Ok(build_theme(album_key, accent, sleeve_path, record_path));
        }
    }

    let cover = image::load_from_memory(art_bytes).map_err(ThemeError::Decode)?;
    let analysis = cover.resize_exact(64, 64, FilterType::Triangle).to_rgb8();
    let accent = color::ensure_contrast(color::pick_accent(&analysis), color::PLINTH, 4.5);

    if !sleeve_path.is_file() {
        let sleeve = render::render_sleeve(&cover, seed_from_key(album_key, SLEEVE_SALT));
        render::write_fast_rgb_png(&sleeve, &sleeve_path)?;
    }
    if !record_path.is_file() {
        let record = render::render_record(&cover, seed_from_key(album_key, RECORD_SALT));
        render::write_fast_rgba_png(&record, &record_path)?;
    }
    write_metadata(&meta_path, accent)?;

    Ok(build_theme(album_key, accent, sleeve_path, record_path))
}

fn build_theme(album_key: &str, accent: Rgb, sleeve_path: PathBuf, record_path: PathBuf) -> Theme {
    let on_accent = color::pick_on_accent(accent);
    Theme {
        album_key: album_key.to_owned(),
        accent,
        on_accent,
        sleeve: Some(Art::File {
            path: sleeve_path.to_string_lossy().into_owned(),
        }),
        record: Some(Art::File {
            path: record_path.to_string_lossy().into_owned(),
        }),
    }
}

fn safe_cache_key(key: &str) -> String {
    let sanitized: String = key
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "unknown".to_owned()
    } else {
        sanitized
    }
}

fn read_metadata(path: &Path) -> Result<Option<Rgb>, io::Error> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut line = String::new();
    BufReader::new(file).read_line(&mut line)?;
    let values: Vec<&str> = line.trim().split(',').collect();
    if values.len() != 4 || values[0] != CACHE_VERSION {
        return Ok(None);
    }
    let channels: Option<Vec<u8>> = values[1..].iter().map(|value| value.parse().ok()).collect();
    let Some(channels) = channels else {
        return Ok(None);
    };
    Ok(Some(Rgb {
        r: channels[0],
        g: channels[1],
        b: channels[2],
    }))
}

fn write_metadata(path: &Path, accent: Rgb) -> Result<(), io::Error> {
    let mut file = File::create(path)?;
    writeln!(
        file,
        "{CACHE_VERSION},{},{},{}",
        accent.r, accent.g, accent.b
    )
}
