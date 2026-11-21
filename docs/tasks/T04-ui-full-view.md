# T04: UI, Full view

| Meta | |
| --- | --- |
| Wave | 1 |
| Depends on | T00 |
| Branch | `feat/t04-ui-full-view` |
| Milestone | M0 |
| PR title | `feat(ui): full-size player view` |

## Goal

Build the Full (360×520) player view in Svelte 5, driven only by `PlayerState` through `ui/src/lib/ipc.ts`. During development it runs against the mock backend in a plain browser.

## Context

`docs/DESIGN.md` → Visual design, User experience. `docs/CONTRACTS.md` §1, §2 and §6. Use the generated types in `ui/src/lib/bindings/`; never redefine them.

## Owns

`ui/src/lib/components/**` (except `modes/`, `queue/`, `onboarding/`), `ui/src/lib/styles/**`, `ui/src/App.svelte`

## Spec

**Visual spec: [`docs/design/full-playing.png`](../design/full-playing.png), [`full-not-running.png`](../design/full-not-running.png), and the measurements and tokens in [`docs/design/README.md`](../design/README.md).** Match them to within 1–2 px. The list below is the behavior; the mockup is the look.

Layout, top to bottom (all sizes in CSS px):

1. **Title bar strip** (28 px): drag region (`data-tauri-drag-region`), with a mode icon on the left (T09 wires it up), grip dots in the center, and a close button on the right that calls `getCurrentWindow().hide()` (T05 owns what hiding does).
2. **Art**: 216×216, centered, 12 px radius, `object-fit: cover`. Art comes from `track.art`: `File` → `convertFileSrc(path)`, `Url` → the URL. With no art, show a neutral placeholder (a music-note SVG on `--surface`).
3. **Meta**: title (17/600, one line, ellipsis; marquee scroll on hover if it's cut off), artists joined with ", " (13/400, 70% opacity).
4. **Progress**: a 4 px bar plus elapsed/total times (11 px, tabular numbers). Extrapolate from `position.sampledAtUnixMs` while `Playing` using `requestAnimationFrame` (pause the loop when hidden). Clicking the bar sends `seek` only when `duration_ms` is known.
5. **Controls**: previous, play/pause (larger, accent-filled), next, and a heart on the right (shown only when `liked !== null`).
6. **Queue placeholder**: an empty `<section id="queue-slot">`. T09 fills it.

Theming: CSS custom properties `--accent`, `--on-accent`, `--text`, `--text-muted` and `--bg-image`, set from `state.theme` with a 300 ms transition. With no theme: dark defaults (`#121212` background, `#1DB954`-free neutral accent `#B3B3B3`; don't use Spotify's brand green).

States:
- `!spotifyRunning`: replace art/meta with "Spotify isn't open" and a button that calls `open_spotify`.
- Errors from `send_command` appear as a 3 s toast at the bottom.

Accessibility: every button has an `aria-label`; Space = play/pause and ←/→ = previous/next when the window has focus; respect `prefers-reduced-motion`.

## Steps

1. Components: `TitleBar`, `Artwork`, `TrackMeta`, `Progress`, `Controls`, `Toast`, `NotRunning`.
2. A `state.svelte.ts` store (Svelte 5 runes) wrapping `ipc.ts`'s subscribe.
3. The theme variables + transitions.
4. Extend `mock.ts` only through its public API (add tracks to its list if needed; T00's file is fine to append to).

## Acceptance criteria

- [ ] `npm run dev` in a browser renders the Full view against the mock, cycling tracks with cross-fading colors
- [ ] At exactly 360×520 it matches `docs/design/full-playing.png` (attach a side-by-side screenshot to the PR); nothing overflows at 125% and 150% Windows scaling
- [ ] The progress bar moves smoothly while playing and freezes when paused
- [ ] The not-running state and the toast can be triggered from the mock (`?mock=notRunning`, `?mock=error`)
- [ ] `npm run check` passes with no TS errors; no `any` in the components
- [ ] Keyboard shortcuts work; buttons have labels

## Out of scope

Card/Strip modes and the queue list (T09), onboarding (T10), the window/tray (T05).
