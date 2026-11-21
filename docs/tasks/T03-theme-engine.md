# T03: Theme engine (album art → worn sleeve, record, accent)

| Meta | |
| --- | --- |
| Wave | 1 |
| Depends on | T00 |
| Branch | `feat/t03-theme-engine` |
| Milestone | M1 |
| PR title | `feat(theme): worn sleeve, record and accent from album art` |

## Goal

Implement `ferric_theme::extract(art_bytes, album_key, cache_dir) -> Theme`. For any cover it produces a readable accent color, a **worn sleeve** image, and a **dusty record** with the cover as its label, matching the "sleeve and record" design. Also commit the album-independent textures.

## Context

- Visual spec: [`docs/design/README.md`](../design/README.md) (read "The idea", "Materials and tokens", "Motion") and the PNGs next to it.
- **Reference implementation:** [`design/tools/textures.py`](../../design/tools/textures.py), functions `worn`, `record`, `plinth`, `card`, `overlay`. Port its look, not necessarily its exact code. Run it (instructions in its docstring) to see the target output.
- `docs/CONTRACTS.md` §1 (`Theme`, `Rgb`, `Art`) and §5.
- Crates: `image` (jpeg, png, webp), `rand` + `rand_chacha` (seeded noise), optionally `imageproc` for line drawing. No other heavy dependencies.

## Owns

`crates/ferric-theme/**`, `ui/src/lib/assets/textures/**`

## Spec

**Accent**
- Downscale to 64×64; 6-color median cut or k-means. Pick the color with the highest `saturation × sqrt(population)` whose lightness is between 0.25 and 0.85. If none qualifies, use `#B3B3B3`.
- Lighten in HSL until the contrast with the plinth `#1E1B18` is ≥ 4.5:1. `on_accent` = black or white, whichever has more contrast with the accent.

**Sleeve** (`theme.sleeve`, 480×480 PNG, `cache_dir/sleeve-<album_key>.png`)
- Fade 14% toward paper `#E8DEC8`.
- Rub the edges and corners toward paper, masked with smooth noise so the wear is uneven.
- Ring wear: a light ring at radius 0.44 × size, centred slightly below the middle, modulated by noise.
- One diagonal crease near a bottom corner, 18 faint scratches, ±6 grain.
- **Seed the randomness from `album_key`**, so the same album always gets the same wear.

**Record** (`theme.record`, 480×480 RGBA PNG, `cache_dir/record-<album_key>.png`)
- Dark disc with groove rings (brightness varies with the radius), a faint radial sheen and a darker rim. Transparent outside the disc.
- Label: the cover, lightly worn, clipped to a circle of radius 0.17 × size, with a spindle hole.
- Dust specks and short curved scratches, so the spin is visible.
- **No baked-in fixed highlight.** The UI draws the rim light as a separate, still layer (see Motion).

**Album-independent textures** (generate once with `textures.py` and commit as PNGs to `ui/src/lib/assets/textures/`):
- `plinth.png` 720×1040
- `grain.png` 720×1040 RGBA, used by the UI at 20% opacity
- `card.png` 656×240

Keep the total under 3 MB; use PNG optimisation (`oxipng`) if needed.

**General**
- Pure and synchronous. The caller uses `spawn_blocking`.
- If both cached files for `album_key` exist, return without decoding the cover again.
- Target: < 120 ms release-mode for a 640×640 JPEG, for both images together.
- `ThemeError`: `Decode`, `Io`.

## Steps

1. Color helpers (`rgb↔hsl`, WCAG luminance and contrast) with tests.
2. Accent pick.
3. Seeded noise helpers (white noise, smooth noise by blur, blobs, scratch lines).
4. `worn_sleeve`, then `record`, with the cache.
5. Generate and commit the three static textures.
6. Example: `cargo run -p ferric-theme --example preview -- cover.jpg` writes `preview.html` showing the sleeve, record (spinning with CSS), accent and grain over the plinth, so it can be compared with `docs/design/full-playing.png`.

## Acceptance criteria

- [ ] For the 3 sample covers from `design/tools/covers.py`, the preview looks like the mockups (side-by-side screenshots in the PR)
- [ ] The accent is ≥ 4.5:1 against `#1E1B18` for those covers plus a greyscale cover and a near-black cover (tests)
- [ ] The same `album_key` gives byte-identical output twice (determinism test), and a second call doesn't rewrite the files (mtime test)
- [ ] Release-mode timing < 120 ms for both images from a 640×640 JPEG (report the number in the PR)
- [ ] `ui/src/lib/assets/textures/` has `plinth.png`, `grain.png`, `card.png`, under 3 MB in total
- [ ] Clippy clean; no `unsafe`

## Out of scope

Calling it from the app (T08), animation and CSS (T04/T09).
