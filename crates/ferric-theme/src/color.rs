use ferric_core::model::Rgb;
use image::RgbImage;

pub const PLINTH: Rgb = Rgb {
    r: 0x1E,
    g: 0x1B,
    b: 0x18,
};
pub const NEUTRAL: Rgb = Rgb {
    r: 0xB3,
    g: 0xB3,
    b: 0xB3,
};
pub const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};
pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };

#[derive(Clone, Copy, Debug)]
pub struct Hsl {
    pub h: f32,
    pub s: f32,
    pub l: f32,
}

/// Picks a vivid accent color from a 64x64 cover using 6-color median cut.
pub fn pick_accent(image: &RgbImage) -> Rgb {
    let mut pixels: Vec<[u8; 3]> = image.pixels().map(|pixel| pixel.0).collect();
    if pixels.is_empty() {
        return NEUTRAL;
    }

    let buckets = median_cut(&mut pixels, 6);
    buckets
        .into_iter()
        .filter_map(|bucket| {
            let color = mean_color(bucket);
            let hsl = rgb_to_hsl(color);
            (hsl.s > 0.02 && (0.25..=0.85).contains(&hsl.l))
                .then_some((color, hsl.s * (bucket.len() as f32).sqrt()))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map_or(NEUTRAL, |(color, _)| color)
}

/// Lightens `color` in HSL until its contrast against `background` meets `target`.
pub fn ensure_contrast(color: Rgb, background: Rgb, target: f32) -> Rgb {
    if contrast_ratio(color, background) >= target {
        return color;
    }
    let original = rgb_to_hsl(color);
    for step in 1..=500 {
        let l = (original.l + (1.0 - original.l) * (step as f32 / 500.0)).min(1.0);
        let candidate = hsl_to_rgb(Hsl { l, ..original });
        if contrast_ratio(candidate, background) >= target {
            return candidate;
        }
    }
    WHITE
}

/// Returns `BLACK` or `WHITE`, whichever has higher contrast on top of `accent`.
pub fn pick_on_accent(accent: Rgb) -> Rgb {
    if contrast_ratio(BLACK, accent) >= contrast_ratio(WHITE, accent) {
        BLACK
    } else {
        WHITE
    }
}

pub fn rgb_to_hsl(rgb: Rgb) -> Hsl {
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

pub fn hsl_to_rgb(hsl: Hsl) -> Rgb {
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

pub fn relative_luminance(color: Rgb) -> f32 {
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

pub fn contrast_ratio(left: Rgb, right: Rgb) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn median_cut(pixels: &mut [[u8; 3]], target_buckets: usize) -> Vec<&mut [[u8; 3]]> {
    let mut buckets = vec![pixels];
    while buckets.len() < target_buckets {
        let Some((index, channel)) = buckets
            .iter()
            .enumerate()
            .filter_map(|(idx, bucket)| widest_channel(bucket).map(|(ch, range)| (idx, ch, range)))
            .max_by_key(|&(_, _, range)| range)
            .map(|(idx, ch, _)| (idx, ch))
        else {
            break;
        };

        let bucket = buckets.swap_remove(index);
        bucket.sort_unstable_by_key(|pixel| pixel[channel]);
        let mid = bucket.len() / 2;
        let (left, right) = bucket.split_at_mut(mid);
        buckets.push(left);
        buckets.push(right);
    }
    buckets
}

fn widest_channel(bucket: &[[u8; 3]]) -> Option<(usize, u8)> {
    if bucket.len() < 2 {
        return None;
    }
    let mut mins = [255_u8; 3];
    let mut maxs = [0_u8; 3];
    for pixel in bucket {
        for ch in 0..3 {
            mins[ch] = mins[ch].min(pixel[ch]);
            maxs[ch] = maxs[ch].max(pixel[ch]);
        }
    }
    let mut best_ch = 0;
    let mut best_range = 0;
    for ch in 0..3 {
        let range = maxs[ch] - mins[ch];
        if range > best_range {
            best_range = range;
            best_ch = ch;
        }
    }
    (best_range > 0).then_some((best_ch, best_range))
}

fn mean_color(bucket: &[[u8; 3]]) -> Rgb {
    let (mut r, mut g, mut b) = (0_u64, 0_u64, 0_u64);
    for pixel in bucket {
        r += u64::from(pixel[0]);
        g += u64::from(pixel[1]);
        b += u64::from(pixel[2]);
    }
    let count = bucket.len() as u64;
    Rgb {
        r: (r / count) as u8,
        g: (g / count) as u8,
        b: (b / count) as u8,
    }
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
    fn contrast_adjustment_lightens_against_plinth() {
        let dark_red = Rgb {
            r: 70,
            g: 20,
            b: 34,
        };
        let adjusted = ensure_contrast(dark_red, PLINTH, 4.5);
        assert!(contrast_ratio(adjusted, PLINTH) >= 4.5);
        assert!(rgb_to_hsl(adjusted).l > rgb_to_hsl(dark_red).l);
    }

    #[test]
    fn median_cut_picks_vivid_color_over_muted_majority() {
        let image = RgbImage::from_fn(64, 64, |x, _| {
            if x < 54 {
                image::Rgb([110, 108, 106])
            } else {
                image::Rgb([216, 92, 54])
            }
        });
        let picked = pick_accent(&image);
        assert!(picked.r > 180 && picked.g < 120);
    }
}
