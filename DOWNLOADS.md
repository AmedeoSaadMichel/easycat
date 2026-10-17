<p align="center"><img src="easycut-app/assets/app-icon.png" width="112" alt="easycat's pink three-eyed cat"></p>

# Download easycat

**Cut. Transform. Meow.** Trim, compress and change video containers on your computer. No accounts, uploads or cloud processing.

## Choose your system

| System | Download | Requirements / status |
| --- | --- | --- |
| macOS · Apple Silicon | [Download DMG](https://github.com/AmedeoSaadMichel/easycut/releases/download/v0.1.0-preview.1/easycat-0.1.0-macos-arm64.dmg) | macOS 15 or later · preview build |
| macOS · Intel | Not available yet | No Intel or universal package has been built or tested |
| Windows · x64 | [Download portable ZIP](https://github.com/AmedeoSaadMichel/easycut/releases/download/v0.1.0-preview.1/easycat-0.1.0-windows-x64.zip) | Windows 10/11 x64 · unsigned preview |
| Linux · x64 | [Download portable tar.gz](https://github.com/AmedeoSaadMichel/easycut/releases/download/v0.1.0-preview.1/easycat-0.1.0-linux-x86_64.tar.gz) | glibc 2.35+ / Ubuntu 22.04+ · desktop with OpenGL |

[Release details and files](https://github.com/AmedeoSaadMichel/easycut/releases/tag/v0.1.0-preview.1) · [SHA-256 checksum](https://github.com/AmedeoSaadMichel/easycut/releases/download/v0.1.0-preview.1/easycat-0.1.0-macos-arm64.dmg.sha256) · [Report a problem](https://github.com/AmedeoSaadMichel/easycut/issues)

Packages are compiled and tested on native CI runners. Windows desktop interaction still needs hands-on validation; Linux startup/rendering is checked under Xvfb. Intel Mac packages will receive download links when ready. While this repository is private, downloads require repository access; public distribution will use a separate downloads repository so the application source can remain private. See the [distribution preparation](docs/private-code-distribution.md).

## Install on macOS

1. Download and open the DMG.
2. Drag **easycat** into **Applications**.
3. Open easycat from Applications.

The package includes FFmpeg, ffprobe, ffplay and their required libraries. Homebrew and Rust are not required. Your original videos are never overwritten.

**Preview status:** this build has an ad-hoc signature and is not notarized with Apple. macOS may display a security warning. If you decide to open it, use the normal macOS Privacy & Security controls; do not disable Gatekeeper system-wide. Public release compliance for bundled third-party components is still under review; this is not a stable release.

### Install with Homebrew

On Apple Silicon with macOS 15 or later, once the public distribution repository is available. The current development-repository tap requires access. Add the personal tap once, then install easycat:

```sh
brew tap AmedeoSaadMichel/easycat https://github.com/AmedeoSaadMichel/easycut.git
brew install --cask easycat
```

[Homebrew requirements, updates and removal](docs/homebrew.md). This installs the same macOS preview and retains the signature limitations above.

## Install on Windows

1. Download the portable ZIP and extract the entire archive.
2. Open the extracted folder and run **easycat.exe**.
3. Keep **Helpers** beside the executable; it contains the FFmpeg tools and DLLs.

The pink cat icon is embedded in the executable. Rust, Homebrew and a separate FFmpeg installation are not required. This unsigned preview may trigger a Windows security warning. An OpenGL-capable graphics driver is required.

[Windows SHA-256 checksum](https://github.com/AmedeoSaadMichel/easycut/releases/download/v0.1.0-preview.1/easycat-0.1.0-windows-x64.zip.sha256)

## Install on Linux

Extract the complete archive and run the launcher from the extracted directory:

```sh
tar -xzf easycat-0.1.0-linux-x86_64.tar.gz
cd easycat-0.1.0-linux-x86_64
./easycat
```

Keep **lib** and **Helpers** in place. Requires a Linux x86_64 desktop with glibc 2.35 or later (Ubuntu 22.04 or compatible), X11 or Wayland, an OpenGL-capable graphics driver and xdg-desktop-portal for file dialogs. FFmpeg, ffprobe and ffplay are included; Rust is not required.

[Linux SHA-256 checksum](https://github.com/AmedeoSaadMichel/easycut/releases/download/v0.1.0-preview.1/easycat-0.1.0-linux-x86_64.tar.gz.sha256)

Windows and Linux currently use software encoding. Hardware-accelerated export is implemented for macOS only. All packages are test previews; source/build provenance is included in each archive. See [native packaging and validation details](docs/portable-packages.md).

## Intel Mac development builds

There is no Intel or universal Mac installer yet. Developers can [build from source](README.md#run) with Rust and the system's FFmpeg tools.

## What's included

- Lossless trimming and compatible container changes.
- Compression presets with automatic sample-based size estimates.
- Video preview, volume, fullscreen and a thumbnail trim timeline.
- An illustrated cosmic interface and the easycat mascot icon.

[Source code](https://github.com/AmedeoSaadMichel/easycut) · [Build and package details](docs/macos-package.md) · [Artwork and font credits](easycut-app/assets/README.md)
