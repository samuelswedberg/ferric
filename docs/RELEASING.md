# Releasing Ferric

Ferric releases are automated. Maintainers do not edit version numbers or upload installers by hand.

## Normal release flow

1. Merge Conventional Commit pull requests into `main`.
2. Release Please updates its open `chore(main): release x.y.z` pull request with the changelog and the same version in the workspace, Tauri config, and UI package.
3. Review and merge that release pull request.
4. Release Please creates the `vX.Y.Z` tag and GitHub Release.
5. In the same workflow run, the `installers` job calls `release.yml`, which builds the NSIS (`-setup.exe`) and MSI installers and uploads them to that release. Versions below `1.0.0` are marked as prereleases.

The build runs from `release-please.yml`, not from a tag-push trigger, because tags and releases created with `GITHUB_TOKEN` don't start other workflows.

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

To rebuild installers for an existing release, or to test the build, run **Actions → Release installers → Run workflow** with the release's tag. The release must already exist.

For a disposable test, create a prerelease on a throwaway tag (not a draft: drafts have no tag yet), run the workflow with that tag, check the assets, then delete it:

```powershell
gh release create v0.0.1-test --prerelease --title "build test" --notes "test"
gh workflow run release.yml -f tag=v0.0.1-test
```

```powershell
gh release delete v0.0.1-test --cleanup-tag --yes
```

Never test with a version that could be mistaken for a real release.

## Hotfixes

For a supported older release, create `release/X.Y` from its tag, apply the smallest fix there, and use Conventional Commits. After its checks pass, create the hotfix release (`gh release create vX.Y.Z --target release/X.Y`) and run **Release installers** with that tag. Do not merge the release branch back into `main`; apply an equivalent fix to `main` separately when it is still relevant.
