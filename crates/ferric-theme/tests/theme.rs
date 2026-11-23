use std::{
    env, fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use ferric_core::model::{Art, Rgb};
use ferric_theme::extract;
use image::{codecs::jpeg::JpegEncoder, DynamicImage, ImageFormat, RgbImage};

const PLINTH: Rgb = Rgb {
    r: 0x1E,
    g: 0x1B,
    b: 0x18,
};
const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};
const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("ferric-theme-{}-{id}", std::process::id()));
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
        }
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn encode(image: RgbImage, format: ImageFormat) -> Vec<u8> {
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

fn cover_ember(size: u32) -> RgbImage {
    let s = size as f32;
    RgbImage::from_fn(size, size, |x, y| {
        let xf = x as f32 / s * 640.0;
        let yf = y as f32 / s * 640.0;
        let t = y as f32 / (size - 1).max(1) as f32;
        let mut rgb = [
            (40.0 * (1.0 - t) + 232.0 * t) as u8,
            (12.0 * (1.0 - t) + 96.0 * t) as u8,
            (30.0 * (1.0 - t) + 44.0 * t) as u8,
        ];
        if (xf - 320.0).hypot(yf - 380.0) <= 130.0 {
            rgb = [255, 196, 92];
        }
        for (i, (h, col)) in [
            (430.0_f32, [70_u8, 20, 34]),
            (500.0_f32, [38, 10, 24]),
            (570.0_f32, [18, 6, 14]),
        ]
        .into_iter()
        .enumerate()
        {
            if yf >= h + 40.0 * (xf / 90.0 + (i as f32) * 2.0).sin() {
                rgb = col;
            }
        }
        image::Rgb(rgb)
    })
}

fn cover_tide(size: u32) -> RgbImage {
    let s = size as f32;
    RgbImage::from_fn(size, size, |x, y| {
        let xf = x as f32 / s * 640.0;
        let yf = y as f32 / s * 640.0;
        let t = y as f32 / (size - 1).max(1) as f32;
        let mut rgb = [
            (8.0 * (1.0 - t) + 18.0 * t) as u8,
            (20.0 * (1.0 - t) + 70.0 * t) as u8,
            (48.0 * (1.0 - t) + 110.0 * t) as u8,
        ];
        if (xf - 490.0).hypot(yf - 120.0) <= 50.0 {
            rgb = [236, 240, 220];
        }
        for i in 0..7_u8 {
            let base_y = 200.0 + f32::from(i) * 60.0;
            if yf >= base_y + 28.0 * (xf / 60.0 + f32::from(i)).sin() {
                let c = [30 + i * 12, 140 + i * 12, 170 + i * 8];
                rgb = if i % 2 == 1 {
                    c
                } else {
                    [
                        (f32::from(c[0]) * 0.7) as u8,
                        (f32::from(c[1]) * 0.7) as u8,
                        (f32::from(c[2]) * 0.7) as u8,
                    ]
                };
            }
        }
        image::Rgb(rgb)
    })
}

fn cover_moss(size: u32) -> RgbImage {
    let s = size as f32;
    RgbImage::from_fn(size, size, |x, y| {
        let xf = x as f32 / s * 640.0;
        let yf = y as f32 / s * 640.0;
        let mut rgb = [226_u8, 222, 206];
        if (60.0..=380.0).contains(&xf) && (60.0..=380.0).contains(&yf) {
            rgb = [58, 92, 60];
        }
        if (xf - 420.0).hypot(yf - 400.0) <= 160.0 {
            rgb = [196, 160, 60];
        }
        if (60.0..=220.0).contains(&xf) && (420.0..=580.0).contains(&yf) {
            rgb = [30, 36, 32];
        }
        if (420.0..=580.0).contains(&xf) && (60.0..=220.0).contains(&yf) && yf - 60.0 <= xf - 420.0
        {
            rgb = [160, 64, 48];
        }
        image::Rgb(rgb)
    })
}

fn fixture(kind: &str, size: u32, format: ImageFormat) -> Vec<u8> {
    let image = match kind {
        "ember" => cover_ember(size),
        "tide" => cover_tide(size),
        "moss" => cover_moss(size),
        "greyscale" => RgbImage::from_fn(size, size, |x, y| {
            let v = ((x + y) % 220 + 20) as u8;
            image::Rgb([v, v, v])
        }),
        "near_black" => RgbImage::from_fn(size, size, |x, y| {
            image::Rgb([(x % 16) as u8, (y % 14) as u8, ((x + y) % 18) as u8])
        }),
        _ => unreachable!(),
    };
    encode(image, format)
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

#[test]
fn all_covers_meet_wcag_contrast_against_plinth() {
    let cache = TestDir::new();
    for kind in ["ember", "tide", "moss", "greyscale", "near_black"] {
        let bytes = fixture(kind, 240, ImageFormat::Png);
        let theme = extract(&bytes, kind, cache.path()).unwrap();
        let ratio = contrast(theme.accent, PLINTH);
        assert!(ratio >= 4.5, "{kind} accent contrast was {ratio}");

        let expected_on_accent = if contrast(BLACK, theme.accent) >= contrast(WHITE, theme.accent) {
            BLACK
        } else {
            WHITE
        };
        assert_eq!(theme.on_accent, expected_on_accent);
    }
}

#[test]
fn greyscale_and_near_black_fall_back_to_neutral() {
    let cache = TestDir::new();
    for kind in ["greyscale", "near_black"] {
        let bytes = fixture(kind, 180, ImageFormat::Png);
        let accent = extract(&bytes, kind, cache.path()).unwrap().accent;
        assert_eq!(
            accent,
            Rgb {
                r: 0xB3,
                g: 0xB3,
                b: 0xB3
            }
        );
    }
}

#[test]
fn sleeve_and_record_outputs_have_expected_dimensions_and_alpha() {
    let cache = TestDir::new();
    let bytes = fixture("ember", 240, ImageFormat::Png);
    let theme = extract(&bytes, "ember", cache.path()).unwrap();

    let Some(Art::File { path: sleeve_path }) = theme.sleeve else {
        panic!("expected sleeve path");
    };
    let Some(Art::File { path: record_path }) = theme.record else {
        panic!("expected record path");
    };

    let sleeve = image::open(&sleeve_path).unwrap();
    assert_eq!((sleeve.width(), sleeve.height()), (480, 480));

    let record = image::open(&record_path).unwrap().to_rgba8();
    assert_eq!((record.width(), record.height()), (480, 480));
    assert_eq!(record.get_pixel(0, 0).0[3], 0);
    assert_eq!(record.get_pixel(240, 240).0[3], 255);
}

#[test]
fn same_album_key_produces_byte_identical_outputs() {
    let dir_a = TestDir::new();
    let dir_b = TestDir::new();
    let bytes = fixture("tide", 240, ImageFormat::Png);

    let theme_a = extract(&bytes, "album-tide", dir_a.path()).unwrap();
    let theme_b = extract(&bytes, "album-tide", dir_b.path()).unwrap();
    assert_eq!(theme_a.accent, theme_b.accent);
    assert_eq!(theme_a.on_accent, theme_b.on_accent);

    let (Some(Art::File { path: sa }), Some(Art::File { path: sb })) =
        (&theme_a.sleeve, &theme_b.sleeve)
    else {
        panic!("expected sleeve paths");
    };
    let (Some(Art::File { path: ra }), Some(Art::File { path: rb })) =
        (&theme_a.record, &theme_b.record)
    else {
        panic!("expected record paths");
    };

    assert_eq!(fs::read(sa).unwrap(), fs::read(sb).unwrap());
    assert_eq!(fs::read(ra).unwrap(), fs::read(rb).unwrap());
}

#[test]
fn cache_hit_does_not_decode_or_rewrite_files() {
    let cache = TestDir::new();
    let bytes = fixture("moss", 240, ImageFormat::Png);
    let first = extract(&bytes, "same-album", cache.path()).unwrap();
    let Some(Art::File { path: sleeve_path }) = &first.sleeve else {
        panic!("expected sleeve")
    };
    let Some(Art::File { path: record_path }) = &first.record else {
        panic!("expected record")
    };
    let sleeve_mtime = fs::metadata(sleeve_path).unwrap().modified().unwrap();
    let record_mtime = fs::metadata(record_path).unwrap().modified().unwrap();
    thread::sleep(Duration::from_millis(20));

    let second = extract(b"not an image", "same-album", cache.path()).unwrap();
    assert_eq!(first.accent, second.accent);
    assert_eq!(
        sleeve_mtime,
        fs::metadata(sleeve_path).unwrap().modified().unwrap()
    );
    assert_eq!(
        record_mtime,
        fs::metadata(record_path).unwrap().modified().unwrap()
    );
}

#[test]
fn stale_cache_version_redraws_existing_images() {
    let cache = TestDir::new();
    let bytes = fixture("moss", 240, ImageFormat::Png);
    let first = extract(&bytes, "old-album", cache.path()).unwrap();
    let Some(Art::File { path: sleeve_path }) = &first.sleeve else {
        panic!("expected sleeve")
    };
    let Some(Art::File { path: record_path }) = &first.record else {
        panic!("expected record")
    };
    let fresh_sleeve = fs::read(sleeve_path).unwrap();

    // Simulate images and metadata left by an older CACHE_VERSION.
    fs::write(sleeve_path, b"old sleeve").unwrap();
    fs::write(record_path, b"old record").unwrap();
    let meta_path = cache.path().join("theme-old-album.meta");
    fs::write(&meta_path, "1,10,20,30").unwrap();

    extract(&bytes, "old-album", cache.path()).unwrap();
    assert_eq!(fs::read(sleeve_path).unwrap(), fresh_sleeve);
    assert_ne!(fs::read(record_path).unwrap(), b"old record");
}

#[test]
#[ignore = "release-mode performance check: cargo test -p ferric-theme --release -- --ignored --nocapture"]
fn release_timing_for_640_jpeg() {
    let bytes = fixture("ember", 640, ImageFormat::Jpeg);
    let cache = TestDir::new();
    let start = Instant::now();
    extract(&bytes, "timing", cache.path()).unwrap();
    let elapsed = start.elapsed();
    eprintln!("640x640 JPEG extraction (sleeve + record): {elapsed:?}");
    assert!(elapsed < Duration::from_millis(120), "elapsed: {elapsed:?}");
}
