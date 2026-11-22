# Ferric

A tiny, album-art-themed miniplayer for Spotify, written in Rust.

> **Status:** approaching v1.0. Release downloads and package-manager listings appear as they become available.

Ferric is an always-on-top miniplayer for Windows that takes its colors from the current album art. It works the moment you install it: no login, no API keys. Signing in to Spotify adds the queue, likes and seeking.

<!-- A short product GIF will replace this note when the first signed release is captured. -->

## Features (planned)

- **Zero setup.** Now playing and play/pause/next/previous come straight from Windows media controls.
- **A record and its sleeve.** The album art becomes a worn sleeve, with a dusty record that spins while the music plays. The accent color comes from the album.
- **Three sizes.** Full (art + queue), Card, and a thin Strip.
- **Queue.** Shows what's coming up; click a track to jump to it (requires signing in).
- **Light.** Rust + Tauri, small installer, close to 0% CPU when idle.
- **Private.** No analytics. Ferric only talks to Spotify, to LRCLIB if lyrics are on, and to GitHub to check for updates.

## Requirements

- Windows 10 or 11 with the Spotify desktop app.
- Optional: a free Spotify developer app (client ID) to sign in. Ferric walks you through creating one in about two minutes.
- Some controls (seek, volume, queue jump) need Spotify Premium when they go through the Web API.

## Install

Ferric supports 64-bit Windows 10 and 11. When the package listings are live, choose one method:

```powershell
winget install Samuelswedberg.Ferric
```

```powershell
scoop bucket add samuelswedberg https://github.com/samuelswedberg/scoop-bucket
scoop install samuelswedberg/ferric
```

Or download the latest NSIS (`-setup.exe`) or MSI installer from [GitHub Releases](https://github.com/samuelswedberg/ferric/releases/latest). Ferric checks GitHub for updates only in builds that contain the maintainer's updater public key, and it never installs an update until you click **Restart**.

## Privacy

Ferric has no analytics, advertising, or telemetry. It contacts Spotify for sign-in, playback controls, metadata, and album art; LRCLIB only when the optional lyrics feature is enabled; and GitHub only to check for app updates. The Spotify developer dashboard and track pages open in your browser only when you click their links. Your Spotify refresh token stays in Windows Credential Manager.

## FAQ

### Why does Ferric ask for a Spotify client ID?

Basic now-playing controls work without signing in. Spotify does not allow open-source desktop apps to safely ship a shared client secret, so the optional Web API features use a free client ID from your own Spotify developer app. Ferric guides you through the two-minute setup and uses PKCE, so there is no client secret to paste into Ferric.

### Do I need Spotify Premium?

No. Play, pause, previous, and next use Windows media controls and work with the Spotify desktop app on a free account. Spotify restricts Web API playback actions such as seeking, volume changes, and queue jumping to Premium accounts.

### Why not use Spotify's own miniplayer?

Ferric is a compact, always-on-top remote built around album art, quick controls, and three window sizes. It is useful when you want a persistent player without keeping Spotify's main window open, and it remains open source and free of analytics.

## Building from source

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

---

Ferric is not affiliated with, endorsed by, or sponsored by Spotify AB. "Spotify" is a trademark of Spotify AB.
