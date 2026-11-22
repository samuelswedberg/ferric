# Releasing Ferric

Ferric releases are automated. Maintainers do not edit version numbers or upload installers by hand.

## Normal release flow

1. Merge Conventional Commit pull requests into `main`.
2. Release Please updates its open `chore(main): release x.y.z` pull request with the changelog and the same version in the workspace, Tauri config, and UI package.
3. Review and merge that release pull request.
4. Release Please creates the `vX.Y.Z` tag and GitHub Release.
5. In the same workflow run, the `installers` job calls `release.yml`, builds NSIS and MSI installers, optionally signs them, and uploads them. Versions below `1.0.0` are prereleases.

The build runs from `release-please.yml`, not from a tag-push trigger, because tags and releases created with `GITHUB_TOKEN` do not start other workflows.

The workflow works without secrets: it emits warnings and publishes unsigned installers without updater artifacts. If both code-signing providers are configured, SignPath takes precedence.

## Enable in-app updates

The updater is deliberately inert unless all three updater secrets are present. A build without the public key does not register the updater plugin, make GitHub update requests, or offer the install command.

Generate the updater key once on a trusted maintainer machine:

```powershell
npm run tauri signer generate --prefix ui
```

Store the private key and password in a password manager. Never commit either value. Add these repository Actions secrets:

- `FERRIC_UPDATER_PUBKEY`: the generated public key
- `TAURI_SIGNING_PRIVATE_KEY`: the complete private key contents
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the private-key password

The release workflow sets `FERRIC_UPDATER_PUBKEY` at compile time and writes the same key plus the GitHub `latest.json` endpoint into the release-only Tauri config. When all three values exist, Tauri creates signed updater bundles and `latest.json`. Ferric checks at startup and every six hours, but downloads and installs only after the user clicks **Restart**.

After configuring the keys, publish two disposable prereleases and install the older one. Confirm the newer release produces the update strip and that clicking **Restart** installs it. Delete both test releases afterward.

## Choose one Windows code-signing provider

### SignPath Foundation

Apply to [SignPath Foundation](https://signpath.org/product/open-source). In SignPath, create a project and artifact configuration that signs the staged `.exe` and `.msi` files, then add:

- `SIGNPATH_API_TOKEN`
- `SIGNPATH_ORGANIZATION_ID`
- `SIGNPATH_PROJECT_SLUG`
- `SIGNPATH_SIGNING_POLICY_SLUG`
- `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`

The workflow uploads the installers as a private workflow artifact, waits for SignPath, and publishes SignPath's returned files.

### Azure Trusted Signing

Create an Azure Trusted Signing account, certificate profile, and service principal with permission to sign. Add:

- `AZURE_TENANT_ID`
- `AZURE_CLIENT_ID`
- `AZURE_CLIENT_SECRET`
- `AZURE_TRUSTED_SIGNING_ENDPOINT`
- `AZURE_TRUSTED_SIGNING_ACCOUNT`
- `AZURE_TRUSTED_SIGNING_PROFILE`

The workflow signs the generated `.exe` and `.msi` files in place with SHA-256 and an RFC 3161 timestamp before upload.

For either provider, verify a test release on Windows with `Get-AuthenticodeSignature` before announcing it. Account creation, identity verification, and certificate approval are manual prerequisites and are not performed by the workflow.

## winget

Templates live in `packaging/winget`. After the first public release:

1. Install Microsoft's `wingetcreate` on a maintainer machine.
2. Replace `VERSION`, `MSI_URL`, `MSI_SHA256`, and `PRODUCT_CODE` in copies of the templates using the published MSI and its SHA-256.
3. Run `winget validate --manifest <prepared-manifest-directory>`.
4. For the first submission, run `wingetcreate new <published-msi-url>` and reconcile its generated files with the templates. For later releases run `wingetcreate update Samuelswedberg.Ferric -v <version> -u <published-msi-url> -s`.
5. Review the generated commit before allowing `wingetcreate` to open the PR to `microsoft/winget-pkgs`.

Do not submit a manifest until the installer URL is public and immutable.

## Scoop

Copy `packaging/scoop/ferric.json` into a `samuelswedberg/scoop-bucket` repository. For the first release, replace the version and placeholder SHA-256, then test from this checkout:

```powershell
scoop install .\packaging\scoop\ferric.json
```

After publishing the bucket:

```powershell
scoop bucket add samuelswedberg https://github.com/samuelswedberg/scoop-bucket
scoop install samuelswedberg/ferric
```

The manifest's `checkver` reads GitHub releases and `autoupdate` derives the NSIS URL. Run `scoop checkver ferric -u` in the bucket for each release and commit the resulting version and hash.

## Testing the release workflow

To rebuild installers for an existing release, run **Actions → Release installers → Run workflow** with the release tag. The release must already exist.

For a disposable test, create a prerelease on a throwaway tag (not a draft), run the workflow, inspect the assets, and then delete it:

```powershell
gh release create v0.0.1-test --prerelease --title "build test" --notes "test"
gh workflow run release.yml -f tag=v0.0.1-test
gh release delete v0.0.1-test --cleanup-tag --yes
```

Never test with a version that could be mistaken for a real release.

## Hotfixes

For a supported older release, create `release/X.Y` from its tag, apply the smallest fix there, and use Conventional Commits. Create the hotfix release (`gh release create vX.Y.Z --target release/X.Y`) and run **Release installers** with that tag. Do not merge the release branch back into `main`; apply an equivalent fix to `main` separately when it is still relevant.
