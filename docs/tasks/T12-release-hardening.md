# T12: v1.0 release hardening

| Meta | |
| --- | --- |
| Wave | 4 |
| Depends on | T06, T10, T11 |
| Branch | `chore/t12-release-hardening` |
| Milestone | M3 |
| PR title | `chore(release): signing, updater UI, winget and Scoop` |

## Goal

Get Ferric ready for strangers to install: signed installers so Windows SmartScreen doesn't warn people off, in-app updates, and listings in winget and Scoop.

## Context

`docs/RELEASING.md` (T06). Code signing options: SignPath Foundation (free for open source; requires an application and an approval wait) or Azure Trusted Signing (paid, cheap). **The maintainer chooses and sets up the account.** This task wires up whichever is chosen and documents both.

## Owns

`packaging/**`, `.github/workflows/release.yml`, `docs/RELEASING.md`. A small updater prompt component at `ui/src/lib/components/update/**`, and `src-tauri/src/updater.rs` (registration append-only).

## Spec

- Signing step in `release.yml`, controlled by which secrets exist (none → unsigned build with a warning, as before).
- Updater: `tauri-plugin-updater` pointing at `https://github.com/samuelswedberg/ferric/releases/latest/download/latest.json`. Check at startup and every 6 h; show a small "Update available: vX.Y.Z · Restart" bar in the UI; never update without a click.
- `packaging/winget/`: a manifest template plus a workflow step (or documented manual step) using `wingetcreate` to open the PR to `microsoft/winget-pkgs` on release.
- `packaging/scoop/ferric.json`: a manifest with `checkver`/`autoupdate` against GitHub releases. Document putting it in a `samuelswedberg/scoop-bucket` repo.
- Privacy check: grep the codebase for every outbound host and confirm it matches the README's privacy statement (Spotify, LRCLIB, GitHub). List them in the PR.
- README: install instructions (winget, Scoop, direct download), a GIF, an FAQ (client ID, Premium, "why not just Spotify's miniplayer").

## Acceptance criteria

- [ ] A test release built from a tag is signed (or the unsigned path is green with a warning), and the PR says which
- [ ] Installing an older test build, then publishing a newer one, shows the update bar; clicking it updates and restarts
- [ ] The winget manifest passes `winget validate`; the Scoop manifest installs from a local path
- [ ] The outbound-host list is in the PR and matches the README
- [ ] The README install section is complete

## Out of scope

macOS/Linux packaging, the Microsoft Store.
