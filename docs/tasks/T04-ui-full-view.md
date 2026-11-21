# T04: UI, Full view ("sleeve and record")

| Meta | |
| --- | --- |
| Wave | 1 |
| Depends on | T00 |
| Branch | `feat/t04-ui-full-view` |
| Milestone | M0 |
| PR title | `feat(ui): sleeve-and-record full view` |

## Goal

Build the Full (360×520) player view in Svelte 5: the worn sleeve, the spinning record and tonearm, the hardware key bank, the slider and the volume knob. Its three states are Playing, Paused and Stopped. It's driven only by `PlayerState` through `ui/src/lib/ipc.ts`, and during development runs against the mock backend in a plain browser.

## Context

- **Visual spec: [`docs/design/README.md`](../design/README.md)** (tokens, Full measurements, States, Motion) and [`full-playing.png`](../design/full-playing.png), [`full-paused.png`](../design/full-paused.png), [`full-stopped.png`](../design/full-stopped.png). Match them to within 1–2 px. The mockups are the look; this file is the behavior.
- The FigCraft file `design/ferric-ui.figcraft` has a clickable prototype of the spin and state changes. Watch it if FigCraft is available.
- `docs/CONTRACTS.md` §1, §2, §6. Use the generated types in `ui/src/lib/bindings/`; never redefine them.
- Textures: `theme.sleeve` / `theme.record` come from T03. Until T03 merges, use the PNGs exported by `design/tools/textures.py` as mock values (put them in `ui/src/lib/mock-assets/`). The static plinth, grain and card textures live in `ui/src/lib/assets/textures/` (T03 commits them; use the same placeholder files until then).

## Owns

`ui/src/lib/components/**` (except `modes/`, `queue/`, `onboarding/`), `ui/src/lib/styles/**`, `ui/src/lib/mock-assets/**`, `ui/src/App.svelte`

## Spec

**Layers, back to front:** plinth → record (rotating layer) → rim light (still) → sleeve → tonearm → text, slider, key bank, knob → queue card slot → grain overlay at 20% (`pointer-events: none`).

**Sleeve and record**
- The sleeve shows `theme.sleeve`, falling back to `track.art` with CSS-only wear (sepia 10%, grain) when no theme exists yet.
- The record shows `theme.record` (Art → `convertFileSrc` for `File`). Its centre and the tonearm angle depend on the state, per the States table in the design README.
- Spin: a CSS `@keyframes` rotation, 1.8 s per turn, on the record layer only. Spin up and down by animating the speed (Web Animations API `playbackRate` from 0 to 1 over 0.4 s, and 1 to 0 over 0.6 s). The rim light never rotates.
- Tonearm: an SVG arm rotating around the pivot. The angle is lerped between the start and end angles from `position / duration`. Lifted = a larger, softer shadow plus a small outward offset. At rest = the rest angle.
- Slide: the record's centre moves between Playing/Paused (236, 140) and Stopped (152, 140) over 0.7 s ease-in-out.

**Text:** title (Bahnschrift SemiBold 24, one line, ellipsis) and artists joined with ", " (Bahnschrift 14). Time "m:ss / m:ss" in Courier New Bold 12, right-aligned. Extrapolate the position from `position.sampledAtUnixMs` while `Playing`, using `requestAnimationFrame`, and pause the loop when hidden.

**Slider:** a recessed slot with the played line in `--accent` and a metal cap. Dragging the cap or clicking the slot sends `seek`, only when `duration_ms` is known; otherwise the cap is disabled.

**Key bank:** four icon-only keys, no text labels:
- previous → `previous`
- play/pause → `playPause`; held down with the pause icon in the accent while `Playing`, up with ▶ otherwise
- next → `next`
- like → `setLiked`; heart filled in the accent when `liked === true`, outlined when `false`, key disabled when `null`

Pressing a key moves it down 2 px over 60 ms.

**Volume knob:** drag up/down or scroll over it → `setVolume`, in steps of 5. The pointer angle shows `volumePct` (−135° to +135°). Disabled (dimmed) when `volumePct === null`.

**Queue card:** render the cardboard card (rotated 0.8°) with an empty `<section id="queue-slot">` inside. T09 fills it with the handwritten rows.

**Window chrome (not in the mockup):**
- The whole plinth is a drag region (`data-tauri-drag-region`), except the controls.
- On hover, two small printed glyphs fade in at the top-left corner, in `--printed` color: × (calls `getCurrentWindow().hide()`) and a mode glyph (T09 wires it up).

**Theming:** set `--accent` and `--on-accent` from `state.theme`, with a 300 ms transition. Cross-fade the sleeve and record images over 0.3 s on track change. With no theme: accent `#B3B3B3`.

**States**
- `!spotifyRunning`: Stopped layout, title "Spotify isn't open", artist line empty, and the play key calls `open_spotify` (aria-label "Open Spotify").
- Errors from `send_command` appear as a 3 s toast: a strip of masking tape along the bottom edge with typed text (Courier New). Not in the mockup; keep it in the same style.

**Accessibility**
- Every key and the knob have an `aria-label` and a visible focus ring (1 px `--printed`).
- Space = play/pause, ←/→ = previous/next, ↑/↓ = volume.
- `prefers-reduced-motion`: no spin and no cross-fades; slide and arm positions jump straight to the end state.

**Performance:** while hidden (`document.visibilityState`), cancel the animation frames and pause CSS animations. Idle CPU should be about 0%.

## Steps

1. Static layout to the pixel, from the mockups, with mock data.
2. The `state.svelte.ts` store (Svelte 5 runes) wrapping `ipc.ts`.
3. The record, tonearm and slide animations, plus reduced motion.
4. Key bank, slider and knob wired to commands.
5. Not-running state, toast, window chrome.
6. Extend `mock.ts` through its public API: add `?mock=paused`, `?mock=stopped`, `?mock=notRunning`, `?mock=error`.

## Acceptance criteria

- [ ] In a browser against the mock, Playing / Paused / Stopped each match their PNG at exactly 360×520 (three side-by-side screenshots in the PR)
- [ ] The record spins at about 33 RPM with a still rim light. Pausing slows it to a stop and lifts the arm; stopping slides the record into the sleeve. A short screen recording goes in the PR.
- [ ] The tonearm moves inward as the track plays; seeking moves it
- [ ] Reduced motion: no spin or cross-fade, and states still read correctly
- [ ] Nothing overflows at 125% and 150% Windows scaling; long titles truncate
- [ ] CPU is about 0% when hidden (DevTools Performance or Task Manager; note the result in the PR)
- [ ] `npm run check` passes; no `any` in components; keyboard shortcuts and aria-labels work

## Out of scope

Card/Strip and the queue rows (T09), onboarding (T10), the window and tray behavior (T05), generating textures (T03).
