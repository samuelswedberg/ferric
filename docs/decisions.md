# Decisions

- 2026-11-22 · Use the Start menu shortcuts already created by Ferric's Tauri NSIS and MSI installers for Windows Search discovery. This keeps installation concerns in the installer and avoids runtime registry or shortcut mutation; a custom startup registration path was rejected as redundant and harder to uninstall cleanly.
