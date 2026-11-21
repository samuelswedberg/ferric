# Releasing Ferric

Ferric releases are automated. Maintainers do not edit version numbers or upload installers by hand.

## Normal release flow

1. Merge Conventional Commit pull requests into `main`.
2. Release Please updates its open `chore(main): release x.y.z` pull request with the changelog and the same version in the workspace, Tauri config, and UI package.
3. Review and merge that release pull request.
4. Release Please creates the `vX.Y.Z` tag and GitHub Release.
5. The tag starts the Windows release workflow. It builds NSIS (`-setup.exe`) and MSI installers and uploads them to that release. Versions below `1.0.0` are marked as prereleases.

The release workflow is intentionally usable by forks without secrets. In that case it logs a warning and uploads unsigned installers without updater artifacts.

## Updater signing key

Generate the updater key once on a trusted maintainer machine:

```powershell
npm run tauri signer generate --prefix ui
```

Store the private key and password in a password manager. Never commit either value. Add them in **Repository settings → Secrets and variables → Actions** as:

- `TAURI_SIGNING_PRIVATE_KEY`: the complete private key contents
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the key password

When both secrets are available, the release build enables Tauri updater artifacts and `tauri-action` adds the signed bundles and `latest.json` to the GitHub Release. The public key will be added to the application updater configuration when the in-app updater lands in T12.

## Testing the workflow

Use a disposable tag only from a branch whose version fields agree:

```powershell
git tag v0.0.1-test
git push origin v0.0.1-test
```

After checking the Actions run and release assets, delete the test release in GitHub and then remove both tags:

```powershell
git push origin --delete v0.0.1-test
git tag --delete v0.0.1-test
```

Do not test by tagging `main` with a version that could be mistaken for a real release.

## Hotfixes

For a supported older release, create `release/X.Y` from its tag, apply the smallest fix there, and use Conventional Commits. Create the hotfix tag from that branch after its checks pass. Do not merge the release branch back into `main`; apply an equivalent fix to `main` separately when it is still relevant.
