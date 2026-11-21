# T06: Release pipeline

| Meta | |
| --- | --- |
| Wave | 1 |
| Depends on | T00 |
| Branch | `ci/t06-release-pipeline` |
| Milestone | M0 |
| PR title | `ci: release-please and tagged installer builds` |

## Goal

Merging the release-please PR turns `main` into a tagged GitHub Release with Windows installers and an updater manifest. No manual steps.

## Context

`CONTRIBUTING.md` → Releases. Tools: `googleapis/release-please-action@v4`, `tauri-apps/tauri-action` (latest v0.x).

## Owns

`.github/workflows/release-please.yml`, `.github/workflows/release.yml`, `release-please-config.json`, `.release-please-manifest.json`, `docs/RELEASING.md`

## Spec

- release-please: release type `simple`, one version for the whole repo, starting at `0.0.0` so the first release is `0.1.0`. It bumps the version in **all** of: root `Cargo.toml` `[workspace.package].version`, `src-tauri/tauri.conf.json` `version`, `ui/package.json` `version` (use `extra-files` with the `generic`/`json`/`toml` updaters). Pre-1.0: `bump-minor-pre-major: true`.
- `release.yml`: triggered on `push: tags: ['v*']`; runs on `windows-latest`; builds NSIS + MSI with `tauri-action`, uploading to the release that release-please created (`releaseId` from the tag). Mark versions `< 1.0.0` as pre-releases.
- Updater: enable `createUpdaterArtifacts` in the build config and produce `latest.json`. Signing key from the secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. The workflow must still **succeed without the secrets** (skip updater artifacts with a warning), so forks can build.
- Permissions: least privilege (`contents: write` only where needed). Pin actions to a major version.
- `docs/RELEASING.md`: how a release happens, how to generate the updater key (`cargo tauri signer generate`), which secrets to add, how to do a hotfix from a `release/X.Y` branch.

## Steps

1. Add the release-please config, manifest and workflow.
2. Add the release workflow.
3. Test in your fork, or with a throwaway `v0.0.1-test` tag on a branch, then delete the test release and tag.

## Acceptance criteria

- [ ] A `feat:` commit on `main` makes release-please open a release PR that bumps all three version fields
- [ ] Pushing a test tag produces a GitHub Release with `.msi` and `-setup.exe` assets (link the test run in the PR)
- [ ] With no signing secrets, the workflow is still green and logs a warning
- [ ] `docs/RELEASING.md` exists and covers the key generation and secrets
- [ ] actionlint (or `gh workflow view`) shows no syntax errors

## Out of scope

Code signing certificates, winget/Scoop (T12), the in-app updater UI (T12).
