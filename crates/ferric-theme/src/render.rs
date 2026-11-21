use std::{
    fs::File,
    io::{self, BufWriter},
    path::Path,
};

use image::{
    codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder},
    imageops::FilterType,
    DynamicImage, ExtendedColorType, ImageEncoder, Rgb, RgbImage, Rgba, RgbaImage,
};

use crate::noise::{
    blur_f32, draw_blobs, draw_filled_circle, draw_scratches, draw_segment, smooth_noise, SeededRng,
};

pub const SLEEVE_SIZE: u32 = 480;
pub const RECORD_SIZE: u32 = 480;

pub fn render_sleeve(cover: &DynamicImage, seed: u64) -> RgbImage {
    let mut rng = SeededRng::new(seed);
    render_worn_square(cover, SLEEVE_SIZE, 0.14, true, true, 1.0, &mut rng)
}

pub fn render_record(cover: &DynamicImage, seed: u64) -> RgbaImage {
    let mut rng = SeededRng::new(seed);
    let size = RECORD_SIZE as usize;
    let s = RECORD_SIZE as f32;
    let center = s * 0.5;

    let label_size = (s * 0.34) as u32;
    let label = render_worn_square(cover, label_size, 0.18, false, false, 0.3, &mut rng);
    let scratches = draw_scratches(size, size, 40, 200, s * 0.03, s * 0.25, 1.0, true, &mut rng);
    let dust = draw_blobs(size, size, 140, 1, 3, 0.6, &mut rng);

    let label_radius = s * 0.17;
    let label_offset = (center - label_radius) as usize;
    let label_dim = label_size as usize;
    let spindle_radius = s * 0.012;
    let rim_start = s * 0.47;
    let rim_width = s * 0.03;
    let sheen_center = s * 0.34;
    let sheen_width = s * 0.18;

    let mut out = RgbaImage::new(RECORD_SIZE, RECORD_SIZE);
    for y in 0..size {
        let dy = y as f32 - center;
        let row = y * size;
        for x in 0..size {
            let dx = x as f32 - center;
            let r = dx.hypot(dy);
            let th = dy.atan2(dx);

            let base = 16.0 + 3.0 * (r * 1.9).sin() + rng.normal(2.5);
            let sheen_r = (r - sheen_center) / sheen_width;
            let sheen = (2.0 * (th - 0.6)).cos().powi(8) * 34.0 * (-sheen_r * sheen_r).exp();
            let rim = ((r - rim_start) / rim_width).clamp(0.0, 1.0);
            let groove = (base + sheen) * (1.0 - rim * 0.4);
            let mut rgb = [groove, groove, groove];

            if r < label_radius
                && x >= label_offset
                && y >= label_offset
                && x - label_offset < label_dim
                && y - label_offset < label_dim
            {
                let lp = label
                    .get_pixel((x - label_offset) as u32, (y - label_offset) as u32)
                    .0;
                rgb = [f32::from(lp[0]), f32::from(lp[1]), f32::from(lp[2])];
            }

            if r < spindle_radius {
                rgb = [30.0, 30.0, 30.0];
            }

            let idx = row + x;
            let scratch = scratches[idx] * 0.35;
            let speck = dust[idx] * 0.55;
            for channel in &mut rgb {
                *channel = *channel * (1.0 - scratch) + 200.0 * scratch;
                *channel = *channel * (1.0 - speck) + 215.0 * speck;
            }

            let alpha = ((center - r) * 2.0).clamp(0.0, 255.0).round() as u8;
            out.put_pixel(
                x as u32,
                y as u32,
                Rgba([
                    rgb[0].round().clamp(0.0, 255.0) as u8,
                    rgb[1].round().clamp(0.0, 255.0) as u8,
                    rgb[2].round().clamp(0.0, 255.0) as u8,
                    alpha,
                ]),
            );
        }
    }
    out
}

fn render_worn_square(
    cover: &DynamicImage,
    size: u32,
    fade: f32,
    ring: bool,
    crease: bool,
    edge: f32,
    rng: &mut SeededRng,
) -> RgbImage {
    let w = size as usize;
    let s = size as f32;
    let resized = cover
        .resize_to_fill(size, size, FilterType::Triangle)
        .to_rgb8();
    let paper = [232.0_f32, 222.0, 200.0];

    let n = smooth_noise(w, w, 2.0, 40.0, rng);
    let crease_mask = if crease {
        let mut buf = vec![0.0_f32; w * w];
        let width = (size / 260).max(1) as f32;
        draw_segment(&mut buf, w, w, s * 0.62, s, s, s * 0.64, width, 1.0);
        Some(blur_f32(&buf, w, w, 1.2))
    } else {
        None
    };
    let scratches = draw_scratches(w, w, 18, 120, s * 0.05, s * 0.30, 1.0, false, rng);

    let edge_span = s * 0.02;
    let corner_span = s * 0.07;
    let ring_r = s * 0.44;
    let ring_cx = s * 0.5;
    let ring_cy = s * 0.52;
    let ring_w1 = s * 0.012;
    let ring_w2 = s * 0.05;

    let mut out = RgbImage::new(size, size);
    for y in 0..w {
        let yf = y as f32;
        let row = y * w;
        for x in 0..w {
            let xf = x as f32;
            let idx = row + x;
            let src = resized.get_pixel(x as u32, y as u32).0;
            let mut rgb = [
                f32::from(src[0]) * (1.0 - fade) + paper[0] * fade,
                f32::from(src[1]) * (1.0 - fade) + paper[1] * fade,
                f32::from(src[2]) * (1.0 - fade) + paper[2] * fade,
            ];

            let nv = n[idx];
            let de = xf.min(yf).min(s - 1.0 - xf).min(s - 1.0 - yf);
            let em = ((edge_span * edge - de) / edge_span + (nv - 0.5) * 0.9)
                .clamp(0.0, 1.0)
                .powf(1.5);

            let cd = xf
                .hypot(yf)
                .min((s - xf).hypot(yf))
                .min(xf.hypot(s - yf))
                .min((s - xf).hypot(s - yf));
            let cm = ((corner_span - cd) / corner_span + (nv - 0.5) * 0.6).clamp(0.0, 1.0);

            let wm = (em.max(cm) * 0.9).clamp(0.0, 1.0);
            for (ch, p) in rgb.iter_mut().zip(paper) {
                *ch = *ch * (1.0 - wm) + p * wm;
            }

            if ring {
                let r = (xf - ring_cx).hypot(yf - ring_cy);
                let t1 = (r - ring_r) / ring_w1;
                let t2 = (r - ring_r) / ring_w2;
                let rm = ((-t1 * t1).exp() * 0.22 + (-t2 * t2).exp() * 0.06) * (0.6 + 0.8 * nv);
                for ch in &mut rgb {
                    *ch = *ch * (1.0 - rm) + 255.0 * rm;
                }
            }

            if let Some(ref mask) = crease_mask {
                let cm2 = mask[idx] * 0.35;
                for ch in &mut rgb {
                    *ch = *ch * (1.0 - cm2) + 255.0 * cm2;
                }
            }

            let sc = scratches[idx] * 0.25;
            let grain = rng.normal(6.0);
            for ch in &mut rgb {
                *ch = *ch * (1.0 - sc) + 245.0 * sc + grain;
            }

            out.put_pixel(
                x as u32,
                y as u32,
                Rgb([
                    rgb[0].round().clamp(0.0, 255.0) as u8,
                    rgb[1].round().clamp(0.0, 255.0) as u8,
                    rgb[2].round().clamp(0.0, 255.0) as u8,
                ]),
            );
        }
    }
    out
}

pub fn write_fast_rgb_png(img: &RgbImage, path: &Path) -> io::Result<()> {
    write_png_inner(
        img.as_raw(),
        img.width(),
        img.height(),
        ExtendedColorType::Rgb8,
        path,
        CompressionType::Fast,
        PngFilter::Sub,
    )
}

pub fn write_fast_rgba_png(img: &RgbaImage, path: &Path) -> io::Result<()> {
    write_png_inner(
        img.as_raw(),
        img.width(),
        img.height(),
        ExtendedColorType::Rgba8,
        path,
        CompressionType::Fast,
        PngFilter::Sub,
    )
}

pub fn write_best_rgb_png(img: &RgbImage, path: &Path) -> io::Result<()> {
    write_png_inner(
        img.as_raw(),
        img.width(),
        img.height(),
        ExtendedColorType::Rgb8,
        path,
        CompressionType::Best,
        PngFilter::Adaptive,
    )
}

pub fn write_best_rgba_png(img: &RgbaImage, path: &Path) -> io::Result<()> {
    write_png_inner(
        img.as_raw(),
        img.width(),
        img.height(),
        ExtendedColorType::Rgba8,
        path,
        CompressionType::Best,
        PngFilter::Adaptive,
    )
}

fn write_png_inner(
    raw: &[u8],
    width: u32,
    height: u32,
    color: ExtendedColorType,
    path: &Path,
    compression: CompressionType,
    filter: PngFilter,
) -> io::Result<()> {
    let file = BufWriter::new(File::create(path)?);
    PngEncoder::new_with_quality(file, compression, filter)
        .write_image(raw, width, height, color)
        .map_err(io::Error::other)
}

/// Generates the album-independent plinth texture (`720x1040` in `textures.py`).
pub fn render_plinth(w: u32, h: u32, seed: u64) -> RgbImage {
    let mut rng = SeededRng::new(seed);
    let (wu, hu) = (w as usize, h as usize);
    let blobs = draw_blobs(wu, hu, 40, 20, 80, 30.0, &mut rng);
    let scratches = draw_scratches(wu, hu, 90, 120, 8.0, 60.0, 1.0, false, &mut rng);

    let base = [30.0_f32, 27.0, 24.0];
    let wf = w as f32;
    let hf = h as f32;
    let mut out = RgbImage::new(w, h);
    for y in 0..hu {
        let yf = y as f32;
        let vy = (yf - hf * 0.3) / (hf * 0.8);
        let row = y * wu;
        for x in 0..wu {
            let xf = x as f32;
            let vx = (xf - wf * 0.5) / (wf * 0.5);
            let vignette = (1.0 - 0.35 * (vx * vx + vy * vy)).clamp(0.5, 1.0);
            let idx = row + x;
            let delta = rng.normal(3.0) + blobs[idx] * 10.0 + scratches[idx] * 40.0;
            out.put_pixel(
                x as u32,
                y as u32,
                Rgb([
                    ((base[0] + delta) * vignette).round().clamp(0.0, 255.0) as u8,
                    ((base[1] + delta) * vignette).round().clamp(0.0, 255.0) as u8,
                    ((base[2] + delta) * vignette).round().clamp(0.0, 255.0) as u8,
                ]),
            );
        }
    }
    out
}

/// Generates the album-independent grain, dust and scratch overlay (`720x1040` RGBA).
pub fn render_grain(w: u32, h: u32, n_dust: usize, seed: u64) -> RgbaImage {
    let mut rng = SeededRng::new(seed);
    let (wu, hu) = (w as usize, h as usize);

    let mut dust = vec![0.0_f32; wu * hu];
    let radii = [1.0_f32, 1.0, 1.0, 2.0, 2.0, 3.0];
    for _ in 0..n_dust {
        let x = rng.usize_range(0, wu) as f32;
        let y = rng.usize_range(0, hu) as f32;
        let d = rng.choose(&radii);
        let fill = rng.usize_range(70, 160) as f32;
        draw_filled_circle(&mut dust, wu, hu, x + d * 0.5, y + d * 0.5, d * 0.5, fill);
    }

    let hair = draw_scratches(wu, hu, 14, 90, 20.0, 90.0, 1.0, true, &mut rng);
    let scr = draw_scratches(wu, hu, 26, 60, 30.0, 200.0, 1.0, false, &mut rng);

    let mut out = RgbaImage::new(w, h);
    for y in 0..hu {
        let row = y * wu;
        for x in 0..wu {
            let idx = row + x;
            let g = rng.normal(1.0);
            let mut rgb = if g > 0.0 { 255_u8 } else { 0_u8 };
            let mut alpha = g.abs() * 16.0;
            let lite = dust[idx].max(hair[idx] * 255.0).max(scr[idx] * 255.0);
            if lite > alpha {
                rgb = 245;
                alpha = lite;
            }
            out.put_pixel(
                x as u32,
                y as u32,
                Rgba([rgb, rgb, rgb, alpha.round().clamp(0.0, 255.0) as u8]),
            );
        }
    }
    out
}

/// Generates the album-independent cardboard queue card texture (`656x240`).
pub fn render_card(w: u32, h: u32, seed: u64) -> RgbImage {
    let mut rng = SeededRng::new(seed);
    let (wu, hu) = (w as usize, h as usize);
    let col = [214.0_f32, 200.0, 172.0];

    let fib_w = (wu / 4).max(1);
    let mut fib_small = vec![0.0_f32; fib_w * hu];
    for y in 0..hu {
        for fx in 0..fib_w {
            let mut sum = 0.0_f32;
            for _ in 0..4 {
                sum += (rng.normal(1.0) * 30.0 + 128.0).clamp(0.0, 255.0);
            }
            fib_small[y * fib_w + fx] = sum * 0.25;
        }
    }
    let blobs = draw_blobs(wu, hu, 10, 30, 90, 25.0, &mut rng);

    let wf = w as f32;
    let hf = h as f32;
    let mut out = RgbImage::new(w, h);
    for y in 0..hu {
        let yf = y as f32;
        let row = y * wu;
        let fib_row = y * fib_w;
        for x in 0..wu {
            let xf = x as f32;
            let idx = row + x;
            let fx = (xf * (fib_w as f32 / wf)).clamp(0.0, (fib_w - 1) as f32);
            let f0 = fx.floor() as usize;
            let f1 = (f0 + 1).min(fib_w - 1);
            let ft = fx - f0 as f32;
            let fib = fib_small[fib_row + f0] * (1.0 - ft) + fib_small[fib_row + f1] * ft;

            let delta = (fib - 128.0) * 0.25 + rng.normal(4.0) - blobs[idx] * 18.0;
            let de = xf.min(yf).min(wf - 1.0 - xf).min(hf - 1.0 - yf);
            let em = ((8.0 - de) / 8.0 + rng.normal(0.3)).clamp(0.0, 1.0) * 0.6;

            out.put_pixel(
                x as u32,
                y as u32,
                Rgb([
                    ((col[0] + delta) * (1.0 - em) + 240.0 * em)
                        .round()
                        .clamp(0.0, 255.0) as u8,
                    ((col[1] + delta) * (1.0 - em) + 240.0 * em)
                        .round()
                        .clamp(0.0, 255.0) as u8,
                    ((col[2] + delta) * (1.0 - em) + 240.0 * em)
                        .round()
                        .clamp(0.0, 255.0) as u8,
                ]),
            );
        }
    }
    out
}
