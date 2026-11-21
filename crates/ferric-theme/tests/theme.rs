use std::{
    fs,
    io::Cursor,
    thread,
    time::{Duration, Instant},
};

use ferric_core::model::{Art, Rgb};
use ferric_theme::extract;
use image::{codecs::jpeg::JpegEncoder, DynamicImage, ImageFormat, RgbImage};

fn fixture(kind: &str, width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let image = RgbImage::from_fn(width, height, |x, y| match kind {
        "dark" => image::Rgb([(x % 24) as u8, (y % 24) as u8, 35]),
        "bright" => image::Rgb([230 + (x % 25) as u8, 220 + (y % 35) as u8, 180]),
        "greyscale" => {
            let value = ((x + y) % 220 + 20) as u8;
            image::Rgb([value, value, value])
        }
        "dominant" => {
            if x < width * 9 / 10 {
                image::Rgb([190, 35, 75])
            } else {
                image::Rgb([15, 210, 170])
            }
        }
        "busy" => image::Rgb([
            ((x * 37 + y * 17) % 256) as u8,
            ((x * 11 + y * 53) % 256) as u8,
            ((x * 71 + y * 7) % 256) as u8,
        ]),
        _ => unreachable!(),
    });
    let mut bytes = Vec::new();
    match format {
        ImageFormat::Jpeg => JpegEncoder::new_with_quality(&mut bytes, 90)
            .encode_image(&DynamicImage::ImageRgb8(image))
            .unwrap(),
        _ => DynamicImage::ImageRgb8(image)
            .write_to(&mut Cursor::new(&mut bytes), format)
            .unwrap(),
    }
    bytes
}

fn luminance(color: Rgb) -> f32 {
    let linear = |channel: u8| {
        let value = f32::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

fn contrast(left: Rgb, right: Rgb) -> f32 {
    let (left, right) = (luminance(left), luminance(right));
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn mean_background(theme: &ferric_core::model::Theme) -> Rgb {
    let Some(Art::File { path }) = &theme.background else {
        panic!("expected cached background file")
    };
    let image = image::open(path).unwrap().to_rgb8();
    let count = u64::from(image.width()) * u64::from(image.height());
    let sums = image.pixels().fold([0_u64; 3], |mut sums, pixel| {
        for (sum, channel) in sums.iter_mut().zip(pixel.0) {
            *sum += u64::from(channel);
        }
        sums
    });
    Rgb {
        r: (sums[0] / count) as u8,
        g: (sums[1] / count) as u8,
        b: (sums[2] / count) as u8,
    }
}

#[test]
fn all_fixtures_have_readable_accents() {
    let cache = tempfile::tempdir().unwrap();
    for kind in ["dark", "bright", "greyscale", "dominant", "busy"] {
        let bytes = fixture(kind, 180, 180, ImageFormat::Png);
        let theme = extract(&bytes, kind, cache.path()).unwrap();
        let ratio = contrast(theme.accent, mean_background(&theme));
        assert!(ratio >= 4.5, "{kind} contrast was {ratio}");
    }
}

#[test]
fn greyscale_uses_neutral_accent() {
    let cache = tempfile::tempdir().unwrap();
    let bytes = fixture("greyscale", 180, 180, ImageFormat::Png);
    let accent = extract(&bytes, "greyscale", cache.path()).unwrap().accent;
    assert_eq!(accent.r, accent.g);
    assert_eq!(accent.g, accent.b);
}

#[test]
fn cache_hit_does_not_decode_or_rewrite_background() {
    let cache = tempfile::tempdir().unwrap();
    let bytes = fixture("dominant", 180, 180, ImageFormat::Png);
    let first = extract(&bytes, "same-album", cache.path()).unwrap();
    let Some(Art::File { path }) = first.background else {
        panic!("expected background")
    };
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    thread::sleep(Duration::from_millis(20));

    let second = extract(b"not an image", "same-album", cache.path()).unwrap();
    assert_eq!(first.accent, second.accent);
    assert_eq!(modified, fs::metadata(path).unwrap().modified().unwrap());
}

#[test]
#[ignore = "release-mode performance check: cargo test -p ferric-theme --release -- --ignored --nocapture"]
fn release_timing_for_640_jpeg() {
    let bytes = fixture("busy", 640, 640, ImageFormat::Jpeg);
    let cache = tempfile::tempdir().unwrap();
    let start = Instant::now();
    extract(&bytes, "timing", cache.path()).unwrap();
    let elapsed = start.elapsed();
    eprintln!("640x640 JPEG extraction: {elapsed:?}");
    assert!(elapsed < Duration::from_millis(40), "elapsed: {elapsed:?}");
}
