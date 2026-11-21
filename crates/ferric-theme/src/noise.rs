use std::f32::consts::{PI, TAU};

/// Hashes an album key and a domain salt into a deterministic 64-bit seed.
pub fn seed_from_key(key: &str, salt: u64) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in key.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    splitmix64(hash ^ salt)
}

fn splitmix64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    state = (state ^ (state >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    state ^ (state >> 31)
}

/// Fast deterministic SplitMix64 PRNG.
pub struct SeededRng {
    state: u64,
    spare_normal: Option<f32>,
}

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed,
            spare_normal: None,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / ((1_u32 << 24) as f32)
    }

    pub fn uniform(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }

    pub fn usize_range(&mut self, min: usize, max_exclusive: usize) -> usize {
        if max_exclusive <= min {
            return min;
        }
        min + (self.next_u64() as usize) % (max_exclusive - min)
    }

    pub fn normal(&mut self, std_dev: f32) -> f32 {
        if let Some(spare) = self.spare_normal.take() {
            return spare * std_dev;
        }
        let u1 = self.next_f32().max(1e-7);
        let u2 = self.next_f32();
        let mag = (-2.0 * u1.ln()).sqrt();
        let angle = TAU * u2;
        self.spare_normal = Some(mag * angle.sin());
        mag * angle.cos() * std_dev
    }

    pub fn choose<T: Copy>(&mut self, items: &[T]) -> T {
        let index = self.usize_range(0, items.len());
        items[index]
    }
}

/// Separable Gaussian blur for small sigma and 3-pass sliding box blur for large sigma.
pub fn blur_f32(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
    if sigma <= 0.05 || w == 0 || h == 0 {
        return src.to_vec();
    }
    if sigma <= 4.0 {
        gaussian_blur_f32(src, w, h, sigma)
    } else {
        let radius = (((12.0 * sigma * sigma / 3.0 + 1.0).sqrt() - 1.0) / 2.0)
            .round()
            .max(1.0) as usize;
        box_blur_f32(src, w, h, radius, 3)
    }
}

fn gaussian_blur_f32(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
    let radius = (sigma * 2.5).ceil() as isize;
    let mut kernel = Vec::with_capacity((radius * 2 + 1) as usize);
    let mut sum = 0.0_f32;
    let two_sigma_sq = 2.0 * sigma * sigma;
    for d in -radius..=radius {
        let weight = (-((d * d) as f32) / two_sigma_sq).exp();
        kernel.push(weight);
        sum += weight;
    }
    for weight in &mut kernel {
        *weight /= sum;
    }

    let mut tmp = vec![0.0_f32; w * h];
    let max_x = (w - 1) as isize;
    for y in 0..h {
        let row_offset = y * w;
        for x in 0..w {
            let mut acc = 0.0_f32;
            for (k_idx, &weight) in kernel.iter().enumerate() {
                let sx = (x as isize + k_idx as isize - radius).clamp(0, max_x) as usize;
                acc += src[row_offset + sx] * weight;
            }
            tmp[row_offset + x] = acc;
        }
    }

    let mut dst = vec![0.0_f32; w * h];
    let max_y = (h - 1) as isize;
    for y in 0..h {
        let row_offset = y * w;
        for x in 0..w {
            let mut acc = 0.0_f32;
            for (k_idx, &weight) in kernel.iter().enumerate() {
                let sy = (y as isize + k_idx as isize - radius).clamp(0, max_y) as usize;
                acc += tmp[sy * w + x] * weight;
            }
            dst[row_offset + x] = acc;
        }
    }
    dst
}

fn box_blur_f32(src: &[f32], w: usize, h: usize, radius: usize, passes: usize) -> Vec<f32> {
    let mut buf = src.to_vec();
    let mut tmp = vec![0.0_f32; w * h];
    let window = (radius * 2 + 1) as f32;
    let inv = 1.0 / window;
    let r_isize = radius as isize;
    let max_x = (w - 1) as isize;
    let max_y = (h - 1) as isize;

    for _ in 0..passes {
        for y in 0..h {
            let row = y * w;
            let mut sum = 0.0_f32;
            for d in -r_isize..=r_isize {
                sum += buf[row + d.clamp(0, max_x) as usize];
            }
            for x in 0..w {
                tmp[row + x] = sum * inv;
                let remove = (x as isize - r_isize).clamp(0, max_x) as usize;
                let add = (x as isize + r_isize + 1).clamp(0, max_x) as usize;
                sum += buf[row + add] - buf[row + remove];
            }
        }

        for x in 0..w {
            let mut sum = 0.0_f32;
            for d in -r_isize..=r_isize {
                sum += tmp[d.clamp(0, max_y) as usize * w + x];
            }
            for y in 0..h {
                buf[y * w + x] = sum * inv;
                let remove = (y as isize - r_isize).clamp(0, max_y) as usize;
                let add = (y as isize + r_isize + 1).clamp(0, max_y) as usize;
                sum += tmp[add * w + x] - tmp[remove * w + x];
            }
        }
    }
    buf
}

/// Generates smooth noise in `[0.0, 1.0]` matching `smooth_noise` in `textures.py`.
pub fn smooth_noise(w: usize, h: usize, blur: f32, amp: f32, rng: &mut SeededRng) -> Vec<f32> {
    let raw: Vec<f32> = (0..w * h)
        .map(|_| (rng.normal(1.0) * amp + 128.0).clamp(0.0, 255.0) / 255.0)
        .collect();
    blur_f32(&raw, w, h, blur)
}

/// Draws `count` soft blurred blobs in `[0.0, 1.0]`.
pub fn draw_blobs(
    w: usize,
    h: usize,
    count: usize,
    rmin: usize,
    rmax: usize,
    blur: f32,
    rng: &mut SeededRng,
) -> Vec<f32> {
    let mut canvas = vec![0.0_f32; w * h];
    for _ in 0..count {
        let cx = rng.usize_range(0, w) as f32;
        let cy = rng.usize_range(0, h) as f32;
        let r = rng.usize_range(rmin, rmax) as f32;
        let val = rng.usize_range(60, 255) as f32 / 255.0;
        draw_filled_circle(&mut canvas, w, h, cx, cy, r, val);
    }
    blur_f32(&canvas, w, h, blur)
}

pub fn draw_filled_circle(buf: &mut [f32], w: usize, h: usize, cx: f32, cy: f32, r: f32, val: f32) {
    let r_sq = r * r;
    let x0 = (cx - r).floor().max(0.0) as usize;
    let x1 = ((cx + r).ceil() as isize).min(w as isize - 1);
    let y0 = (cy - r).floor().max(0.0) as usize;
    let y1 = ((cy + r).ceil() as isize).min(h as isize - 1);
    if x1 < x0 as isize || y1 < y0 as isize {
        return;
    }
    for y in y0..=(y1 as usize) {
        let dy = y as f32 - cy;
        let dy_sq = dy * dy;
        let row = y * w;
        for x in x0..=(x1 as usize) {
            let dx = x as f32 - cx;
            if dx * dx + dy_sq <= r_sq {
                buf[row + x] = val;
            }
        }
    }
}

/// Draws `count` straight or subtly curved hairline scratches in `[0.0, 1.0]`.
#[allow(clippy::too_many_arguments)]
pub fn draw_scratches(
    w: usize,
    h: usize,
    count: usize,
    alpha: usize,
    lmin: f32,
    lmax: f32,
    width: f32,
    curvy: bool,
    rng: &mut SeededRng,
) -> Vec<f32> {
    let mut canvas = vec![0.0_f32; w * h];
    let min_alpha = alpha / 2;
    for _ in 0..count {
        let x = rng.uniform(0.0, w as f32);
        let y = rng.uniform(0.0, h as f32);
        let angle = rng.uniform(0.0, PI);
        let len = rng.uniform(lmin, lmax);
        let val = rng.usize_range(min_alpha, alpha.max(min_alpha + 1)) as f32 / 255.0;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let mut prev_x = x;
        let mut prev_y = y;
        for step in 1..12 {
            let t = step as f32 / 11.0;
            let bend = if curvy {
                (t * 3.0).sin() * len * 0.08
            } else {
                0.0
            };
            let nx = x + cos_a * len * t - sin_a * bend;
            let ny = y + sin_a * len * t + cos_a * bend;
            draw_segment(&mut canvas, w, h, prev_x, prev_y, nx, ny, width, val);
            prev_x = nx;
            prev_y = ny;
        }
    }
    canvas
}

#[allow(clippy::too_many_arguments)]
pub fn draw_segment(
    buf: &mut [f32],
    w: usize,
    h: usize,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    width: f32,
    val: f32,
) {
    let pad = width * 0.5 + 1.5;
    let min_x = (x0.min(x1) - pad).floor().max(0.0) as usize;
    let max_x = ((x0.max(x1) + pad).ceil() as isize).min(w as isize - 1);
    let min_y = (y0.min(y1) - pad).floor().max(0.0) as usize;
    let max_y = ((y0.max(y1) + pad).ceil() as isize).min(h as isize - 1);
    if max_x < min_x as isize || max_y < min_y as isize {
        return;
    }

    let vx = x1 - x0;
    let vy = y1 - y0;
    let len_sq = vx * vx + vy * vy;
    let half_w = width * 0.5 + 0.35;

    for y in min_y..=(max_y as usize) {
        let py = y as f32 + 0.5;
        let row = y * w;
        for x in min_x..=(max_x as usize) {
            let px = x as f32 + 0.5;
            let t = if len_sq > 1e-6 {
                (((px - x0) * vx + (py - y0) * vy) / len_sq).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let dx = px - (x0 + vx * t);
            let dy = py - (y0 + vy * t);
            let dist = (dx * dx + dy * dy).sqrt();
            let cov = (half_w - dist).clamp(0.0, 1.0);
            if cov > 0.0 {
                let idx = row + x;
                buf[idx] = buf[idx].max(val * cov);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_from_key_is_deterministic_and_distinct() {
        let a1 = seed_from_key("album-ember", 1);
        let a2 = seed_from_key("album-ember", 1);
        let b = seed_from_key("album-tide", 1);
        let c = seed_from_key("album-ember", 2);
        assert_eq!(a1, a2);
        assert_ne!(a1, b);
        assert_ne!(a1, c);
    }

    #[test]
    fn seeded_rng_produces_identical_sequences() {
        let mut r1 = SeededRng::new(42);
        let mut r2 = SeededRng::new(42);
        for _ in 0..32 {
            assert_eq!(r1.next_u64(), r2.next_u64());
            assert_eq!(r1.normal(6.0).to_bits(), r2.normal(6.0).to_bits());
        }
    }
}
