# Homebrew installation

The repository includes a custom Homebrew Cask for the macOS Apple Silicon preview. It installs the same self-contained DMG as the direct download, including FFmpeg, ffprobe and ffplay. Rust and a separate FFmpeg installation are not required.

Requirements: an Apple Silicon Mac, macOS 15 Sequoia or later, and Homebrew. This cask does not install Windows, Linux or Intel Mac builds.

## Install

The public distribution repository contains the project-maintained cask. Add it once:

```sh
brew tap AmedeoSaadMichel/easycat https://github.com/AmedeoSaadMichel/easycat-releases.git
brew trust --cask amedeosaadmichel/easycat/easycat
brew install --cask easycat
```

The trust command authorizes only the easycat cask, not every package or command in the tap. Recent Homebrew versions require explicit trust for non-official taps or their individual packages. See [Homebrew Tap Trust](https://docs.brew.sh/Tap-Trust).

If you previously added the private development tap, run `brew untap AmedeoSaadMichel/easycat` first, then add the public URL above. The explicit URL is required because the repository is named `easycat-releases`. This is a personal tap, not a listing in the official Homebrew catalog.

## First launch on macOS

This preview has an ad-hoc signature and is not notarized with Apple. Installation can succeed while macOS still requires separate approval at first launch. This applies to both the DMG and Homebrew installations.

1. Open **easycat** from **Applications**.
2. If macOS blocks it because the developer cannot be verified or the app is not notarized, open **System Settings → Privacy & Security**.
3. Find the notice for **easycat**, click **Open Anyway**, then confirm the opening. Authenticate if macOS asks.

If the easycat notice is missing, try opening the app again, then return to Privacy & Security. Do not disable Gatekeeper system-wide. See [Apple's instructions](https://support.apple.com/en-us/102445).

## Installation troubleshooting

- **No available formula with the name "easycat":** on a new machine, add the personal tap using the commands above. easycat is not listed in the official Homebrew catalog.
- **Unknown command: brew --cask:** use `brew install --cask easycat`; `--cask` belongs after `install`.
- **Refusing to load cask from untrusted tap:** run `brew trust --cask amedeosaadmichel/easycat/easycat`, then retry the installation.
- **Warnings about other taps**, such as `gromgit/fuse`, `oven-sh/bun` or `wickenico/wailbrew`, are unrelated to easycat. You do not need to trust those taps to install easycat.

The message `easycat was successfully installed!` confirms installation. macOS first-launch approval is a separate step.

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

Update `Casks/easycat.rb` only after uploading and validating a new macOS release. Set `version` to `APP_VERSION,PREVIEW_SUFFIX` (currently `0.1.0,preview.5`); the URL uses these values for the release tag and DMG filename. Replace `sha256` with the checksum of the actual published DMG. Never replace an existing release asset without also updating its checksum.

The cask deliberately skips automatic release discovery: preview updates must be checked before changing the pinned version. Keep the architecture and minimum macOS requirement aligned with the packaged application's `Info.plist`. Validate changes with Homebrew's cask loader and `brew style Casks/easycat.rb`, then verify the downloaded DMG against the pinned SHA-256.

## Official catalog submission

See the [official Homebrew submission preparation and outstanding requirements](homebrew-submission.md). The current ad-hoc signed preview is not ready for the official catalog.
