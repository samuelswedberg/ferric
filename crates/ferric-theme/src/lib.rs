use std::{
    cmp::Reverse,
    collections::HashMap,
    fs::{self, File},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use ferric_core::model::{Art, Rgb, Theme};
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, ImageEncoder, RgbImage};

const NEUTRAL: Rgb = Rgb {
    r: 0xB3,
    g: 0xB3,
    b: 0xB3,
};
const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};
const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
const CACHE_VERSION: &str = "1";

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("could not decode album art: {0}")]
    Decode(#[source] image::ImageError),
    #[error("theme cache I/O failed: {0}")]
    Io(#[from] io::Error),
}

/// Derives a readable color theme and cached background from encoded album art.
///
/// This function is CPU-bound and synchronous; callers in async code should use
/// `spawn_blocking`.
pub fn extract(art_bytes: &[u8], album_key: &str, cache_dir: &Path) -> Result<Theme, ThemeError> {
    fs::create_dir_all(cache_dir)?;
    let cache_key = safe_cache_key(album_key);
    let background_path = cache_dir.join(format!("bg-{cache_key}.jpg"));
    let metadata_path = cache_dir.join(format!("bg-{cache_key}.theme"));

    if background_path.is_file() {
        if let Some(cached) = read_metadata(&metadata_path)? {
            return Ok(build_theme(
                album_key,
                cached.accent,
                cached.mean_background,
                background_path,
            ));
        }
    }

    let cover = image::load_from_memory(art_bytes).map_err(ThemeError::Decode)?;
    let analysis = cover.resize_exact(64, 64, FilterType::Triangle).to_rgb8();
    let fitted = cover
        .resize_to_fill(90, 130, FilterType::Triangle)
        .to_rgb8();
    let mean_background = rendered_mean(&fitted);
    let accent = ensure_contrast(pick_accent(&analysis), mean_background, 4.5);

    if !background_path.is_file() {
        render_background(fitted, &background_path)?;
    }
    write_metadata(
        &metadata_path,
        CachedTheme {
            accent,
            mean_background,
        },
    )?;

    Ok(build_theme(
        album_key,
        accent,
        mean_background,
        background_path,
    ))
}

#[derive(Clone, Copy)]
struct CachedTheme {
    accent: Rgb,
    mean_background: Rgb,
}

fn build_theme(album_key: &str, accent: Rgb, mean_background: Rgb, path: PathBuf) -> Theme {
    let on_accent = if contrast_ratio(BLACK, accent) >= contrast_ratio(WHITE, accent) {
        BLACK
    } else {
        WHITE
    };

    Theme {
        album_key: album_key.to_owned(),
        accent,
        on_accent,
        text: WHITE,
        text_muted: blend(WHITE, mean_background, 0.70),
        background: Some(Art::File {
            path: path.to_string_lossy().into_owned(),
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

fn render_background(small: RgbImage, path: &Path) -> Result<(), ThemeError> {
    let mut darkened = image::imageops::blur(&small, 6.0);
    for pixel in darkened.pixels_mut() {
        for channel in &mut pixel.0 {
            *channel = (f32::from(*channel) * 0.35).round() as u8;
        }
    }
    // The image is already heavily blurred, so nearest-neighbor preserves the
    // intended appearance while avoiding an unnecessary second convolution.
    let output = image::imageops::resize(&darkened, 360, 520, FilterType::Nearest);
    let file = File::create(path)?;
    JpegEncoder::new_with_quality(file, 80)
        .write_image(
            output.as_raw(),
            output.width(),
            output.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(image_error_as_io)?;
    Ok(())
}

/// Computes the mean of the pixels before blur/upscale. Both operations preserve
/// the mean to rounding tolerance, avoiding a second full-size allocation.
fn rendered_mean(fitted: &RgbImage) -> Rgb {
    let (mut r, mut g, mut b) = (0_u64, 0_u64, 0_u64);
    for pixel in fitted.pixels() {
        r += u64::from(pixel[0]);
        g += u64::from(pixel[1]);
        b += u64::from(pixel[2]);
    }
    let count = u64::from(fitted.width()) * u64::from(fitted.height());
    Rgb {
        r: ((r as f64 / count as f64) * 0.35).round() as u8,
        g: ((g as f64 / count as f64) * 0.35).round() as u8,
        b: ((b as f64 / count as f64) * 0.35).round() as u8,
    }
}

fn image_error_as_io(error: image::ImageError) -> io::Error {
    io::Error::other(error)
}

fn read_metadata(path: &Path) -> Result<Option<CachedTheme>, io::Error> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut line = String::new();
    BufReader::new(file).read_line(&mut line)?;
    let values: Vec<&str> = line.trim().split(',').collect();
    if values.len() != 7 || values[0] != CACHE_VERSION {
        return Ok(None);
    }
    let channels: Option<Vec<u8>> = values[1..].iter().map(|value| value.parse().ok()).collect();
    let Some(channels) = channels else {
        return Ok(None);
    };
    Ok(Some(CachedTheme {
        accent: Rgb {
            r: channels[0],
            g: channels[1],
            b: channels[2],
        },
        mean_background: Rgb {
            r: channels[3],
            g: channels[4],
            b: channels[5],
        },
    }))
}

fn write_metadata(path: &Path, cached: CachedTheme) -> Result<(), io::Error> {
    let mut file = File::create(path)?;
    writeln!(
        file,
        "{CACHE_VERSION},{},{},{},{},{},{}",
        cached.accent.r,
        cached.accent.g,
        cached.accent.b,
        cached.mean_background.r,
        cached.mean_background.g,
        cached.mean_background.b
    )
}

#[derive(Default)]
struct Bucket {
    count: u32,
    r: u64,
    g: u64,
    b: u64,
}

fn pick_accent(image: &RgbImage) -> Rgb {
    let mut histogram: HashMap<u16, Bucket> = HashMap::new();
    for pixel in image.pixels() {
        let key = (u16::from(pixel[0] >> 3) << 10)
            | (u16::from(pixel[1] >> 3) << 5)
            | u16::from(pixel[2] >> 3);
        let bucket = histogram.entry(key).or_default();
        bucket.count += 1;
        bucket.r += u64::from(pixel[0]);
        bucket.g += u64::from(pixel[1]);
        bucket.b += u64::from(pixel[2]);
    }
    let mut colors: Vec<(Rgb, u32)> = histogram
        .into_values()
        .map(|bucket| {
            let count = u64::from(bucket.count);
            (
                Rgb {
                    r: (bucket.r / count) as u8,
                    g: (bucket.g / count) as u8,
                    b: (bucket.b / count) as u8,
                },
                bucket.count,
            )
        })
        .collect();
    colors.sort_unstable_by_key(|color| Reverse(color.1));

    colors
        .into_iter()
        .take(6)
        .filter_map(|(color, population)| {
            let hsl = rgb_to_hsl(color);
            (hsl.s > 0.02 && (0.25..=0.85).contains(&hsl.l))
                .then_some((color, hsl.s * (population as f32).sqrt()))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map_or(NEUTRAL, |(color, _)| color)
}

#[derive(Clone, Copy, Debug)]
struct Hsl {
    h: f32,
    s: f32,
    l: f32,
}

fn rgb_to_hsl(rgb: Rgb) -> Hsl {
    let r = f32::from(rgb.r) / 255.0;
    let g = f32::from(rgb.g) / 255.0;
    let b = f32::from(rgb.b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) / 2.0;
    if delta <= f32::EPSILON {
        return Hsl { h: 0.0, s: 0.0, l };
    }
    let s = delta / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    Hsl { h, s, l }
}

fn hsl_to_rgb(hsl: Hsl) -> Rgb {
    let chroma = (1.0 - (2.0 * hsl.l - 1.0).abs()) * hsl.s;
    let x = chroma * (1.0 - ((hsl.h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hsl.h {
        h if h < 60.0 => (chroma, x, 0.0),
        h if h < 120.0 => (x, chroma, 0.0),
        h if h < 180.0 => (0.0, chroma, x),
        h if h < 240.0 => (0.0, x, chroma),
        h if h < 300.0 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = hsl.l - chroma / 2.0;
    Rgb {
        r: ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        g: ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        b: ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    }
}

fn ensure_contrast(color: Rgb, background: Rgb, target: f32) -> Rgb {
    if contrast_ratio(color, background) >= target {
        return color;
    }
    let original = rgb_to_hsl(color);
    for step in 1..=1000 {
        let delta = step as f32 / 1000.0;
        let darker = Hsl {
            l: (original.l - delta).max(0.0),
            ..original
        };
        let lighter = Hsl {
            l: (original.l + delta).min(1.0),
            ..original
        };
        let dark_rgb = hsl_to_rgb(darker);
        let light_rgb = hsl_to_rgb(lighter);
        let dark_ok = contrast_ratio(dark_rgb, background) >= target;
        let light_ok = contrast_ratio(light_rgb, background) >= target;
        match (dark_ok, light_ok) {
            (true, true) => {
                return if contrast_ratio(dark_rgb, background)
                    >= contrast_ratio(light_rgb, background)
                {
                    dark_rgb
                } else {
                    light_rgb
                };
            }
            (true, false) => return dark_rgb,
            (false, true) => return light_rgb,
            (false, false) => {}
        }
    }
    color
}

fn blend(foreground: Rgb, background: Rgb, alpha: f32) -> Rgb {
    let channel = |foreground: u8, background: u8| {
        (f32::from(foreground) * alpha + f32::from(background) * (1.0 - alpha)).round() as u8
    };
    Rgb {
        r: channel(foreground.r, background.r),
        g: channel(foreground.g, background.g),
        b: channel(foreground.b, background.b),
    }
}

fn relative_luminance(color: Rgb) -> f32 {
    let linearize = |channel: u8| {
        let value = f32::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b)
}

fn contrast_ratio(left: Rgb, right: Rgb) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wcag_luminance_and_contrast_known_pairs() {
        assert!((relative_luminance(BLACK) - 0.0).abs() < 0.0001);
        assert!((relative_luminance(WHITE) - 1.0).abs() < 0.0001);
        assert!((contrast_ratio(BLACK, WHITE) - 21.0).abs() < 0.001);
        assert!((contrast_ratio(Rgb { r: 0, g: 0, b: 255 }, WHITE) - 8.59).abs() < 0.02);
    }

    #[test]
    fn hsl_round_trip_is_close() {
        for color in [
            BLACK,
            WHITE,
            Rgb {
                r: 12,
                g: 98,
                b: 210,
            },
            Rgb {
                r: 240,
                g: 33,
                b: 88,
            },
        ] {
            let actual = hsl_to_rgb(rgb_to_hsl(color));
            assert!((i16::from(actual.r) - i16::from(color.r)).abs() <= 1);
            assert!((i16::from(actual.g) - i16::from(color.g)).abs() <= 1);
            assert!((i16::from(actual.b) - i16::from(color.b)).abs() <= 1);
        }
    }

    #[test]
    fn contrast_adjustment_meets_target() {
        let background = Rgb {
            r: 30,
            g: 30,
            b: 30,
        };
        let adjusted = ensure_contrast(
            Rgb {
                r: 50,
                g: 70,
                b: 90,
            },
            background,
            4.5,
        );
        assert!(contrast_ratio(adjusted, background) >= 4.5);
    }
}
