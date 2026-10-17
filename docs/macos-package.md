# macOS local test package

`python3 scripts/package_macos.py` packages the release binary from `cargo build --release -p easycut-app` and the installed FFmpeg tools into a self-contained Apple Silicon application and compressed DMG. The current build requires macOS 15 or later, derived from the minimum deployment versions of the included Mach-O binaries.

The application launcher uses the bundled Helpers directory, independent of the Finder PATH. Non-system dynamic libraries are copied recursively into Frameworks and relinked through loader-relative paths. SDL3 is included for the SDL2 compatibility layer used by ffplay. The source binaries' SHA-256 hashes, package receipts, formula metadata, available license files and SBOMs are preserved under Contents/Resources. The packaging script verifies no non-system absolute dependencies remain and applies an ad-hoc signature to the bundle and each executable/library.

The DMG contains easycat.app, an Applications shortcut and English installation notes. Drag the app into Applications to install. There is no Developer ID certificate or notarization for this local build. Public distribution review, complete source-offer obligations for third-party components and release notarization are not completed by this script. Do not treat this local test artifact as an approved public release.

The staging directory is `target/macos-package`; an existing staging app or output DMG is intentionally not overwritten. Rename previous generated artifacts before packaging another build. System disk-image services may require execution outside a restricted sandbox.

Verification performed: strict recursive code-signature verification; a generated 0.3-second H.264/AAC clip encoded and probed using bundled executables with a system-only PATH; audio playback using bundled ffplay with dummy SDL audio/video drivers; disk-image verification. This is an arm64 build, not an Intel or universal build.
