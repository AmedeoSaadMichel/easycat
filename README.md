# easycat

<p align="center"><img src="easycat-app/assets/app-icon.png" width="112" alt="easycat cat icon"></p>

A local Rust desktop app for trimming and compressing videos, with a cosmic native UI. Videos stay on your computer.

## Download

**[Choose your operating system and download easycat →](DOWNLOADS.md)**

Preview downloads are available for macOS Apple Silicon and Intel, Windows x64 and Linux x86_64. See the download page for requirements, installation steps and validation limits.

Install the macOS preview for your architecture through the [project Homebrew tap](docs/homebrew.md) from the public distribution repository. See [open-source distribution](docs/open-source-distribution.md). Preparation for the official catalog is tracked in the [Homebrew submission checklist](docs/homebrew-submission.md).

### Install with Homebrew

```sh
brew tap AmedeoSaadMichel/easycat https://github.com/AmedeoSaadMichel/easycat-releases.git
brew trust --cask amedeosaadmichel/easycat/easycat
brew install --cask easycat
```

The trust command authorizes only the easycat cask, not every package or command in the tap. Recent Homebrew versions require explicit trust for non-official taps or their individual packages. See [Homebrew Tap Trust](https://docs.brew.sh/Tap-Trust).

After installation, open easycat from Applications. If macOS blocks this unnotarized preview, go to **System Settings → Privacy & Security → Open Anyway** for easycat and confirm. See the [first-launch steps and troubleshooting](DOWNLOADS.md#first-launch-on-macos-dmg-and-homebrew).

## Run

FFmpeg and ffprobe must be available on PATH. ffplay enables preview audio.

```sh
cargo run -p easycat-app
```

The GUI supports drag and drop, opening files, thumbnail selection, portrait previews, export profiles and stream-copy container conversion. Use Space for playback, arrows to step, and I/O to set selection boundaries. Start and end fields accept seconds or minutes:seconds. Use the four-corner preview button or F for fullscreen playback; Escape returns to the editor.

The workspace contains easycat-core, easycat-cli and easycat-app; the command-line executable is easycat.

```sh
cargo run -p easycat-cli -- --help
cargo run -p easycat-cli -- remux input.mp4 output.mkv
```

## Output size estimates

The initial **Rough estimate** is a heuristic. After you select a compression profile and stop changing settings for 700 ms, easycat automatically refines it by encoding samples in the background. It measures only the selected profile, caches the result, and invalidates it when trim, hardware or speed changes.

- Selections up to 30 seconds are measured as one continuous video sample.
- Longer selections use five six-second windows across the selected range.
- All samples use one backend from the same fallback chain used for export.
- Audio is projected at the configured encoding bitrate; container overhead and scene variation can still change the final size.
- **Sample-based estimate** is more informed, not an exact size guarantee. **Target size** shows the requested bitrate-based target.
- Playback delays automatic measurement; export cancels measurement immediately. Profiles can also be measured manually.
- If refinement fails, the UI keeps the rough estimate and exposes the error in a tooltip.

Copy/remux estimates remain proportional to the source size; variable bitrate and keyframe boundaries can affect them.

## License

easycat is licensed under the [MIT License](LICENSE). The application source repository is [AmedeoSaadMichel/easycat](https://github.com/AmedeoSaadMichel/easycat). Third-party components retain their own licenses; see [third-party notices](distribution/THIRD-PARTY-NOTICES.md) and the [distribution guide](docs/open-source-distribution.md).

## Development

The project language is English for UI, CLI, errors, documentation, code comments and tests.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

[Artwork and font credits](easycat-app/assets/README.md) · [8-bit VideoToolbox benchmark](docs/benchmarks/8bit-videotoolbox.md)

## Changing containers with optional tracks

Change container copies encoded media without re-encoding; it does not compress the video. The panel lists incompatible tracks by their original FFmpeg stream index, type and codec/tag. Timecode/data streams, subtitles and attachments can be explicitly excluded when they are incompatible with the selected destination. Exclusion consent resets when the destination or operation changes.

Every video and audio stream is always retained. If one is incompatible, choose a different container or use Compress. Export remains disabled until every blocker is resolved; the core enforces the same policy independently of the GUI.

The CLI preserves every track by default and reports blockers. To explicitly omit a reported incompatible optional track, use its index (zero-based, as printed in the error):

```sh
cargo run -p easycat-cli -- remux camera.mov output.mkv --exclude-track 2
```

Repeat `--exclude-track` for additional incompatible optional tracks. Unknown indices, compatible-track exclusions and video/audio exclusions are rejected. Generated MOV/timecode integration coverage verifies that default conversion is rejected, explicit omission succeeds, and video plus both audio tracks keep their original encoded-data hashes.

Preview 4 and later support [update notices and a per-platform minimum build](docs/app-updates.md), with offline use preserved when the check fails. Preview 3 and earlier require one manual update to receive future notifications.
