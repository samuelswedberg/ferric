# Ferric

A tiny, album-art-themed miniplayer for Spotify, written in Rust.

> **Status:** pre-alpha. Nothing to install yet. See [docs/tasks](docs/tasks/README.md) for the build plan.

Ferric is an always-on-top miniplayer for Windows that takes its colors from the current album art. It works the moment you install it: no login, no API keys. Signing in to Spotify adds the queue, likes and seeking.

<!-- TODO: hero GIF once M1 lands -->

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
