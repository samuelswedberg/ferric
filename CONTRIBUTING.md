# Contributing to Ferric

Thanks for helping. This file covers how work flows into the repo. For the architecture, see [docs/DESIGN.md](docs/DESIGN.md). For the shared types every part of the app relies on, see [docs/CONTRACTS.md](docs/CONTRACTS.md).

## Prerequisites

- Rust stable (see `rust-toolchain.toml` once it exists), target `x86_64-pc-windows-msvc`
- Node 20+ and npm
- Windows 10/11 with WebView2 (preinstalled on Windows 11)
- Tauri prerequisites: <https://v2.tauri.app/start/prerequisites/>

```sh
npm ci --prefix ui
cargo tauri dev        # or: npm run tauri dev --prefix ui
```

## Branching: GitHub flow

- `main` is always releasable. It is protected: changes land only through a pull request with green CI.
- Branch off `main` for each change and keep the branch short-lived:
  - `feat/<short-name>`: new behavior
  - `fix/<short-name>`: bug fix
  - `chore/`, `docs/`, `ci/`, `refactor/`: everything else
  - Task branches use the name given in the task file, e.g. `feat/t01-media-watcher`.
- Rebase on `main` before asking for review. Don't merge `main` into your branch.
- PRs are **squash-merged**, so the PR title becomes the commit on `main`.

There is no `develop` branch. Releases are tags, not branches (see below).

## Commit and PR titles: Conventional Commits

```
feat(media): watch Spotify's SMTC session
fix(ui): strip mode clips long titles
chore(ci): cache cargo registry
```

Scopes: `core`, `media`, `spotify`, `theme`, `app`, `ui`, `ci`, `docs`, `release`.
Use `feat!:` or a `BREAKING CHANGE:` footer for breaking changes.

## Pull request checklist

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `npm run check --prefix ui` and `npm run build --prefix ui`
- [ ] Changes to `crates/ferric-core` (the contracts) are in their own PR, with `docs/CONTRACTS.md` updated in the same PR
- [ ] New user-visible behavior is mentioned in the PR description

## Releases

Releases are automated with [release-please](https://github.com/googleapis/release-please):

1. As PRs merge, release-please keeps a "chore(main): release x.y.z" PR open, with the version bump and CHANGELOG.
2. Merging that PR creates the tag `vX.Y.Z`.
3. The tag triggers the release workflow, which builds the signed installers and attaches them to the GitHub Release.

If an old release ever needs a hotfix, cut `release/X.Y` from that tag, fix it there, and tag from it.

## Code of conduct

Be kind and assume good intent. Maintainers may remove comments or contributions that aren't respectful.
