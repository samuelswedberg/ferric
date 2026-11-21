# T09: UI, Card and Strip modes, handwritten queue, volume readout

| Meta | |
| --- | --- |
| Wave | 2 |
| Depends on | T00, T04 |
| Branch | `feat/t09-ui-modes-queue` |
| Milestone | M1 |
| PR title | `feat(ui): card and strip modes, handwritten queue, scroll volume` |

## Goal

Add the Card and Strip sizes in the "sleeve and record" style, the handwritten queue on the Full view's card, switching between modes, and scroll-to-change volume with its temporary readout.

## Context

- **Visual spec: [`docs/design/README.md`](../design/README.md)** (Card, Strip, States, Motion) and [`card.png`](../design/card.png), [`card-volume.png`](../design/card-volume.png), [`strip.png`](../design/strip.png), [`strip-hover.png`](../design/strip-hover.png), [`strip-volume.png`](../design/strip-volume.png), [`full-playing.png`](../design/full-playing.png) (queue card). The mockups are the look; this file is the behavior.
- Builds on T04's components (sleeve, record, tonearm, key bank, slider, grain). Reuse them with size props; don't copy them.
- `set_mode` and `set_queue_visible` come from `ipc.ts`.

## Owns

`ui/src/lib/components/modes/**`, `ui/src/lib/components/queue/**`, `ui/src/App.svelte` (taking over from T04)

## Spec

**Card (360×120):** sleeve 96 px with the record peeking out and a mini tonearm. Title, artist and time; slider; key bank (previous, play/pause, next, like). The record spins and the arm follows progress, as in Full. Paused/Stopped work the same way, scaled down. No volume knob.

**Strip (360×36):** a tiny sleeve with the record edge peeking out, title and artist (one line; marquee when it overflows), time on the right, and a 3 px progress line along the bottom edge in `--accent`.
**On hover** (150 ms): the time fades out and a fade gradient comes in from the right, showing a 4-key tray (previous, play/pause, next, like).

**Scroll volume (Card and Strip):**
- Each scroll notch over the window → `setVolume` ±5, only when `volumePct !== null`.
- While scrolling:
  - Card: the slider fill turns `--volume` (cream), the cap shows the volume, and the time reads "VOL 60".
  - Strip: the bottom line turns cream and shows the volume, and the time reads "VOL 60".
- 1.5 s after the last notch, everything animates back to progress (200 ms).
- `volumePct === null` (signed out or no Premium): show "VOL —" for 1 s and change nothing.

**Queue (Full view, inside T04's `#queue-slot` on the cardboard card):**
- Handwritten in Segoe Print, `--pencil` color, rotated with the card. Header "side one"; then up to 3 rows: "n. Title" with the duration right-aligned.
- `queue === null`: one handwritten line "sign in to see what's next" with a small key-styled "Sign in" button. The button does nothing yet (`// T10`).
- Clicking a row → `JumpTo { queueIndex }` (only the first 10 can be jumped to). Hovering a row underlines it like a pencil stroke.
- Right-click a row → a small menu: "Copy link" (`https://open.spotify.com/track/<id>`), "Open in Spotify" (`spotify:track:<id>`).
- Call `set_queue_visible(true/false)` when the card enters or leaves view, and on mode changes and document visibility.

**Switching modes:** double-click empty plinth to cycle Full → Card → Strip → Full, or use the mode glyph T04 puts in the corner. Call `set_mode`, and cross-fade the content (opacity and scale, 200 ms). The window resize itself is T05's. Respect reduced motion.

## Acceptance criteria

- [ ] Card, Card · scrolling, Strip, Strip · hover and Strip · scrolling each match their PNG at exact size (side-by-side screenshots in the PR)
- [ ] The handwritten queue matches `full-playing.png`, and the signed-out line shows when `queue === null`
- [ ] Scroll volume: 20 notches go 0 → 100, the readout returns to progress 1.5 s after the last notch, and "VOL —" shows when the volume is unknown
- [ ] `set_queue_visible` fires correctly when switching to Card and back (logged in the mock)
- [ ] Mode cycling works by double-click and by the glyph; reduced motion is respected
- [ ] `npm run check` is clean

## Out of scope

Onboarding and sign-in screens (T10), the hotkey for switching modes (T11), textures (T03).
