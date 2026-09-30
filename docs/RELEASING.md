# Releasing Azeradio

The addon and the tray app version independently. Nothing is manual: pushing a tag runs the release.

## Tag formats

- `addon/vX.Y.Z` → `.github/workflows/addon-release.yml`: stamps `## Version:` in both `.toc` files, zips `addon/Azeradio`, publishes a GitHub Release with the zip.
- `tray/vX.Y.Z` → `.github/workflows/tray-release.yml`: stamps the version into `tray/Cargo.toml` and `tray/tauri.conf.json`, builds the Windows installer on `windows-latest`, publishes a GitHub Release with the installer.

The version lives in the tag. Do not bump version files by hand; the workflows stamp them at build time.

## Latest downloads

Each release build also refreshes a rolling `latest` release per product (`addon/latest`, `tray/latest`) with stable asset names (`Azeradio.zip`, `Azeradio-setup.exe`, `Azeradio.msi`). README and SETUP link straight at those URLs, so users always get the newest build without knowing the version. The workflows own these tags; don't move them by hand.

## Cutting a release

1. Merge everything to `main` first. Releases build from the tagged commit, so push before tagging.
2. Tag only what changed. An addon-only fix gets just an `addon/v*` tag.
3. Create annotated tags and push them:
   ```
   git tag -a addon/v0.2.0 -m "azeradio addon v0.2.0"
   git tag -a tray/v0.2.0 -m "azeradio tray v0.2.0"
   git push origin addon/v0.2.0 tray/v0.2.0
   ```
   Tagger identity comes from the repo git config, which must be `vocino <65593+vocino@users.noreply.github.com>` (see the repo's git config; never override it).
4. Watch the runs in the Actions tab. The tray build takes a while (full Tauri Windows build); the addon zip finishes fast.

## Before tagging

- Addon: `luac -p addon/Azeradio/core.lua` must pass.
- Tray: the Rust must compile. Smart App Control on the dev machine blocks locally compiled build scripts, so CI is the check: tray-check runs unit tests on every push, and the full build runs on tag. If either fails, see below.
- Docs covering changed behavior (`docs/SETUP.md`, `README.md`) must match the tagged code.
- Commit messages stay conventional (`feat:`, `fix:`, `docs:`, `chore:`); the commit hook enforces this.

## Redoing a release

Tags are cheap. If a build fails or the wrong commit got tagged:

```
git push --delete origin tray/v0.2.0
git tag -d tray/v0.2.0
```

Delete the GitHub Release too if one was created, then fix and re-tag. Tags move; `main` history does not get force-pushed to fix a release.

## Rebuilding without a new version

Both workflows also run on `workflow_dispatch`, so any release can be rebuilt from the Actions tab without cutting a new tag.
