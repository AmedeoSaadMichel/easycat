<p align="center"><img src="easycut-app/assets/app-icon.png" width="112" alt="easycat's pink three-eyed cat"></p>

# Download easycat

**Cut. Transform. Meow.** Trim, compress and change video containers on your computer. No accounts, uploads or cloud processing.

## Choose your system

| System | Download | Requirements / status |
| --- | --- | --- |
| macOS · Apple Silicon | [Download DMG](https://github.com/AmedeoSaadMichel/easycut/releases/download/v0.1.0-preview.1/easycat-0.1.0-macos-arm64.dmg) | macOS 15 or later · preview build |
| macOS · Intel | Not available yet | No Intel or universal package has been built or tested |
| Windows · x64 | Not available yet | Native package and testing still pending |
| Linux · x64 | Not available yet | Native package and testing still pending |

[Release details and files](https://github.com/AmedeoSaadMichel/easycut/releases/tag/v0.1.0-preview.1) · [SHA-256 checksum](https://github.com/AmedeoSaadMichel/easycut/releases/download/v0.1.0-preview.1/easycat-0.1.0-macos-arm64.dmg.sha256) · [Report a problem](https://github.com/AmedeoSaadMichel/easycut/issues)

Only packages actually built and tested are linked here. Other platforms will receive download links when their packages are ready. While this repository is private, downloads require repository access; making the repository public will make these releases accessible as well.

## Install on macOS

1. Download and open the DMG.
2. Drag **easycat** into **Applications**.
3. Open easycat from Applications.

The package includes FFmpeg, ffprobe, ffplay and their required libraries. Homebrew and Rust are not required. Your original videos are never overwritten.

**Preview status:** this build has an ad-hoc signature and is not notarized with Apple. macOS may display a security warning. If you decide to open it, use the normal macOS Privacy & Security controls; do not disable Gatekeeper system-wide. Public release compliance for bundled third-party components is still under review; this is not a stable release.

## Windows, Linux and Intel Mac development builds

There are no ready-to-install packages for these systems yet. Developers can [build from source](README.md#run) with Rust and the system's FFmpeg tools. These development instructions are not a substitute for a tested installer.

## What's included

- Lossless trimming and compatible container changes.
- Compression presets with automatic sample-based size estimates.
- Video preview, volume, fullscreen and a thumbnail trim timeline.
- An illustrated cosmic interface and the easycat mascot icon.

[Source code](https://github.com/AmedeoSaadMichel/easycut) · [Build and package details](docs/macos-package.md) · [Artwork and font credits](easycut-app/assets/README.md)
