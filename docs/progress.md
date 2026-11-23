# Progress

- 2026-11-22 · T0003: added a tested Restart Ferric tray action, automatic release-build Start Menu shortcut registration (`src-tauri/src/shortcut.rs`) with COM unit tests, and complete Tauri Windows bundle icon/NSIS metadata in `tauri.conf.json`; a human tray restart + Windows Search smoke test and review remain.
- 2026-11-22 · T0002: moved Spotify track/artist links onto song title and artist text, made sleeve/record toggle play/pause with soft hover scrim, tonearm hover preview, and full resting pose on pause, replaced Play/Pause key with Shuffle/Repeat cycle (`prev, next, shuffle/repeat, like`) across Full/Card/Strip, and wired Spotify Web API shuffle/repeat/artist_id end-to-end. Left: merge PR #33 once CI passes.
