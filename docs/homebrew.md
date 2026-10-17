# Homebrew installation

The repository includes a custom Homebrew Cask for the macOS Apple Silicon preview. It installs the same self-contained DMG as the direct download, including FFmpeg, ffprobe and ffplay. Rust and a separate FFmpeg installation are not required.

Requirements: an Apple Silicon Mac, macOS 15 Sequoia or later, and Homebrew. This cask does not install Windows, Linux or Intel Mac builds.

## Install

Once this repository is public:

```sh
brew tap AmedeoSaadMichel/easycat https://github.com/AmedeoSaadMichel/easycut.git
brew install --cask amedeosaadmichel/easycat/easycat
```

The explicit repository URL is required because the repository is named `easycut`, rather than Homebrew's conventional `homebrew-easycat`. This is a project-maintained tap, not a listing in the official Homebrew cask repository.

While the repository is private, the public installation commands cannot download the release anonymously. Use the authenticated GitHub release download described on the [download page](../DOWNLOADS.md) instead.

This preview has an ad-hoc signature and is not notarized with Apple. Homebrew installation does not change that: macOS may require approval through System Settings > Privacy & Security. Do not disable Gatekeeper system-wide.

## Upgrade or remove

When a new package has been published and the cask has been updated:

```sh
brew update
brew upgrade --cask amedeosaadmichel/easycat/easycat
```

To remove the app and optionally the tap:

```sh
brew uninstall --cask amedeosaadmichel/easycat/easycat
brew untap amedeosaadmichel/easycat
```

## Maintaining the cask

Update `Casks/easycat.rb` only after uploading and validating a new macOS release. Set `version` to `APP_VERSION,PREVIEW_SUFFIX` (currently `0.1.0,preview.1`); the URL uses these values for the release tag and DMG filename. Replace `sha256` with the checksum of the actual published DMG. Never replace an existing release asset without also updating its checksum.

The cask deliberately skips automatic release discovery: preview updates must be checked before changing the pinned version. Keep the architecture and minimum macOS requirement aligned with the packaged application's `Info.plist`. Validate changes with Homebrew's cask loader and `brew style Casks/easycat.rb`, then verify the downloaded DMG against the pinned SHA-256.

## Official catalog submission

See the [official Homebrew submission preparation and outstanding requirements](homebrew-submission.md). The current private, ad-hoc signed preview is not ready for the official catalog.
