# T09: UI, Card and Strip modes, queue list, themes

| Meta | |
| --- | --- |
| Wave | 2 |
| Depends on | T00, T04 |
| Branch | `feat/t09-ui-modes-queue` |
| Milestone | M1 |
| PR title | `feat(ui): card and strip modes, queue list, mono and plain themes` |

## Goal

Add the two smaller sizes, the queue list in Full mode, the switch between modes, and the Mono and Plain visual themes.

## Context

`docs/DESIGN.md` → User experience, Visual design. Builds on T04's components (reuse `Artwork`, `TrackMeta`, `Progress`, `Controls`; don't copy them). `set_mode` and `set_queue_visible` come from `ipc.ts`.

## Owns

`ui/src/lib/components/modes/**`, `ui/src/lib/components/queue/**`, `ui/src/App.svelte` (taking over from T04)

## Spec

**Visual spec: [`card.png`](../design/card.png), [`strip.png`](../design/strip.png), [`strip-hover.png`](../design/strip-hover.png), [`full-playing.png`](../design/full-playing.png) (queue), [`full-signed-out.png`](../design/full-signed-out.png), [`full-mono.png`](../design/full-mono.png), and the measurements in [`docs/design/README.md`](../design/README.md).** The mockups are the look; the list below is the behavior.

- **Card** (360×120): 88 px art on the left, then title/artists, progress and controls on the right.
- **Strip** (360×36): "Title • Artist" in one line with a marquee when it overflows, and a 2 px progress line along the bottom edge. The controls fade in on hover (150 ms).
- Switch modes: double-click the empty background cycles Full → Card → Strip → Full; also a small mode button in the title strip. Call `set_mode`; animate the content (opacity/scale 200 ms). The window resize itself is T05's.
- **Queue** (Full mode, under the controls; fills the remaining height, scrolls):
  - `queue === null` → a "Sign in to see your queue" row with a button that does nothing yet (`// T10`).
  - Rows: 32 px, one line: title 13/600 + artists 12/400 muted (truncates) + duration on the right; hover background as in the mockup. Bottom 20 px fades out. The first 10 rows are clickable → `JumpTo { queueIndex }`; later rows look disabled, with a tooltip "Too far ahead to jump to".
  - Right-click a row → a small menu: "Copy link" (`https://open.spotify.com/track/<id>`), "Open in Spotify" (`spotify:track:<id>`).
  - Call `set_queue_visible(true/false)` when the queue enters or leaves view (IntersectionObserver plus document visibility).
- **Themes**: `data-theme="cover|mono|plain"` on `<html>`. Mono = greyscale filter on the art and background, neutral accent. Plain = no background image, `#181818`. Temporary picker: `?theme=` in the URL, until T11 adds config.
- Volume: scroll over the window → `SetVolume` ±5 (only when `volumePct !== null`).

## Acceptance criteria

- [ ] All three modes match their mockups at their exact sizes (side-by-side screenshots in the PR)
- [ ] Double-click cycles modes, and the mode button works; content animates and respects reduced motion
- [ ] The queue renders 20 mock tracks, scrolls smoothly, and the jump/disabled rules apply
- [ ] `set_queue_visible` fires correctly when switching to Card and back (log it in the mock)
- [ ] Mono and Plain look right for a bright cover and a dark cover
- [ ] `npm run check` is clean

## Out of scope

Onboarding/sign-in screens (T10), the hotkey for switching modes (T11).
