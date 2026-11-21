# T03: Theme engine (album art → colors + background)

| Meta | |
| --- | --- |
| Wave | 1 |
| Depends on | T00 |
| Branch | `feat/t03-theme-engine` |
| Milestone | M1 |
| PR title | `feat(theme): derive accent and blurred background from album art` |

## Goal

Implement `ferric_theme::extract(art_bytes, album_key, cache_dir) -> Theme`: a readable accent color and a blurred, darkened background image for any cover.

## Context

`docs/CONTRACTS.md` §1 (`Theme`, `Rgb`) and §5. Visual rules are in `docs/DESIGN.md` → Visual design. Crates: `image` (jpeg, png, webp), plus `color-thief` **or** a small k-means of your own (no other heavy deps).

## Owns

`crates/ferric-theme/**`

## Spec

- Decode the image, then downscale to 64×64 for color analysis.
- Palette: the 6 most common colors. Accent = the one with the highest `saturation × sqrt(population)` whose lightness is between 0.25 and 0.85. If none qualifies, use a neutral `#B3B3B3`.
- Background: downscale the cover to 90×130 (cover-fit, centered), Gaussian blur sigma 6, multiply brightness by 0.35, upscale to 360×520, and save as `cache_dir/bg-<album_key>.jpg` (quality 80). If the file exists, reuse it without decoding again.
- Contrast: compute WCAG relative luminance. Lighten or darken the accent in HSL until its contrast with the **mean background color** is ≥ 4.5:1. `on_accent` = black or white, whichever has more contrast with the accent. `text` = `#FFFFFF`, `text_muted` = white at about 70% blended over the mean background (as solid `Rgb`).
- Pure and synchronous. The caller runs it in `spawn_blocking`. Target: < 40 ms for a 640×640 JPEG in release mode.
- `ThemeError`: `Decode`, `Io`.

## Steps

1. Color helpers (`rgb↔hsl`, luminance, contrast) with unit tests against known WCAG pairs.
2. Palette + accent pick.
3. Background render + cache.
4. Add `tests/fixtures/` with 5 small covers (generate them in code, or use public-domain images; record the license in `tests/fixtures/README.md`): dark, bright, greyscale, one dominant color, busy.
5. `cargo bench` or a `#[test] #[ignore]` timing check.
6. Example: `cargo run -p ferric-theme --example preview -- cover.jpg` writes `preview.html` that shows the background, the accent and sample text.

## Acceptance criteria

- [ ] All 5 fixtures produce a theme whose accent has ≥ 4.5:1 contrast with the mean background (asserted in tests)
- [ ] The greyscale fixture gives a neutral accent, not a random tint
- [ ] A second call with the same `album_key` doesn't rewrite the background file (check the mtime in a test)
- [ ] Release-mode timing < 40 ms for a 640×640 JPEG on the dev machine (report the number in the PR)
- [ ] Clippy clean; no `unsafe`

## Out of scope

Calling it from the app (T08), CSS/animation (T04/T09), Mono/Plain themes (T09 handles them in CSS).
