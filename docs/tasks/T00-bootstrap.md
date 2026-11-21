# T00: Bootstrap workspace and contracts

| Meta | |
| --- | --- |
| Wave | 0 (run alone, before anything else) |
| Depends on | none |
| Branch | `feat/t00-bootstrap` |
| Milestone | M0 |
| PR title | `feat: bootstrap workspace, contracts and CI` |

## Goal

Create the empty-but-compiling skeleton of the whole app, with the shared contracts **fully implemented**. Every later task then fills in its own crate or folder without touching anyone else's.

## Context

Read `docs/DESIGN.md` and **`docs/CONTRACTS.md`**. The contracts doc is the spec for `ferric-core`. Implement it exactly: same names, same serde attributes.

## Owns

Everything it creates. After merge it keeps only `crates/ferric-core/**`.

## Spec

Repo layout to create:

```
Cargo.toml                 # [workspace] members + [workspace.package] + [workspace.dependencies]
rust-toolchain.toml        # channel = "stable", components = ["rustfmt", "clippy"]
crates/
  ferric-core/             # FULL implementation of CONTRACTS.md §1–4 + unit tests
  ferric-media/            # stubs (§5): fns return Err(Unimplemented) / Handled::Unsupported
  ferric-spotify/          # stubs (§5): mod auth, mod poller, mod controller, error.rs
  ferric-theme/            # stub (§5)
src-tauri/                 # Tauri 2 app: lib.rs + main.rs + empty window.rs/tray.rs/app.rs/commands.rs/router.rs/theming.rs
  tauri.conf.json          # productName "Ferric", identifier "dev.ferric.app", one window "main" 360x520, decorations false, transparent true, alwaysOnTop true
  capabilities/default.json
ui/                        # Vite + Svelte 5 + TypeScript
  src/lib/ipc.ts           # typed wrappers for every command/event in CONTRACTS.md §6 + mock backend
  src/lib/bindings/        # ts-rs output (generated, committed)
  src/lib/mock.ts          # fake PlayerState that cycles 3 tracks every 5 s, for browser dev
  src/App.svelte           # renders JSON of the state; T04 replaces it
.github/workflows/ci.yml   # fmt, clippy -D warnings, test, ui check+build; windows-latest; cargo + npm cache
.env.example               # FERRIC_LOG=info
```

Workspace dependencies to pin (latest stable versions at the time of the task): `tokio` (rt-multi-thread, macros, sync, time), `serde` (derive), `serde_json`, `thiserror`, `anyhow`, `async-trait`, `tracing`, `tracing-subscriber`, `ts-rs`, `tauri` 2, `tauri-build` 2.

`ipc.ts` picks the mock backend when `window.__TAURI_INTERNALS__` is undefined, so `npm run dev --prefix ui` works in a plain browser.

ts-rs: add a test (`cargo test -p ferric-core export_bindings`) that writes the bindings, and a CI step that fails if `git diff --exit-code ui/src/lib/bindings` shows changes.

## Steps

1. Scaffold the Tauri 2 app (`npm create tauri-app@latest`, Svelte + TS), then move the frontend to `ui/` and point `tauri.conf.json` `build.frontendDist` / `devUrl` / `beforeDevCommand` at it.
2. Turn the root into a cargo workspace; add the four crates.
3. Implement `ferric-core` from CONTRACTS.md, with tests: `Store::update` doesn't notify when nothing changed; serde round-trips for every enum, matching the JSON tag shapes in the doc.
4. Add the stubs for §5, so every signature in the contracts exists and compiles.
5. Write `ipc.ts`, `mock.ts`, and a minimal `App.svelte` that subscribes and shows the state as JSON.
6. Add the CI workflow and `rust-toolchain.toml`.

## Acceptance criteria

- [ ] `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass
- [ ] `npm ci && npm run check && npm run build` pass in `ui/`
- [ ] `cargo tauri dev` opens a borderless 360×520 always-on-top window showing the default `PlayerState` as JSON
- [ ] `npm run dev --prefix ui` in a plain browser shows the mock state changing every 5 s
- [ ] `ui/src/lib/bindings/` holds a generated TS type for every type in CONTRACTS.md §1–2
- [ ] Every public item in CONTRACTS.md §1–5 exists with the exact name and signature
- [ ] CI workflow is green on the PR

## Out of scope

Any real behavior in media/spotify/theme, styling, tray, hotkeys, the release workflow.
