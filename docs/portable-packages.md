# Windows and Linux preview packages

The manually dispatched `Windows and Linux packages` GitHub Actions workflow builds on Windows Server 2022 x64 and Ubuntu 22.04 x86_64. It runs workspace unit/integration tests, builds optimized native binaries and runs packaged-tool smoke tests for probing, trimming, remuxing, software compression and audio playback. The Linux package additionally opens the packaged GUI under Xvfb with software OpenGL and captures a screenshot.

## Package layout

Windows is a ZIP containing `easycat.exe`, `Helpers`, the mascot PNG and third-party notices. The executable has the cat ICO embedded through the Windows SDK resource compiler; no resource crate is added. The packaged GUI prepends its adjacent Helpers directory to PATH only when ffmpeg, ffprobe and ffplay are all present. Release builds use the Windows GUI subsystem and launch FFmpeg tools with CREATE_NO_WINDOW to avoid console popups. Extract the complete archive; do not move the executable out of its folder.

Linux is a tar.gz containing the `easycat` launcher, `easycat-bin`, `Helpers`, `lib`, a desktop-entry template and notices. The launcher sets its own PATH and LD_LIBRARY_PATH before starting the GUI. Desktop GL/Wayland/X11 drivers and xdg-desktop-portal remain system dependencies; they are not replaced by bundled GPU drivers. Run `./easycat` from the extracted folder. The build targets glibc 2.35+ (Ubuntu 22.04 or a compatible x86_64 distribution).

The Linux desktop entry is a template for optional installation; its Exec and Icon entries need the installed location or PATH/icon-theme integration. Extraction and launching the shell entry point require no desktop-entry installation.

## Dependencies and integrity

`scripts/ffmpeg-bundles.json` pins a dated BtbN/FFmpeg-Builds release URL and SHA-256 for each platform. Packaging fails if a download checksum changes or any of the three tools is missing. The shared build's bin/lib trees are preserved, along with license files, documentation and source/build provenance. Each output archive gets its own SHA-256 sidecar. See the upstream build repository and the included license information for FFmpeg/dependency details; public release compliance review is still pending for these previews.

No user videos or application data are uploaded by easycat. Build-time downloads occur only in the packaging workflow. Windows and Linux export currently use software encoders; the hardware toggle is disabled because the implemented GPU export backend is specific to macOS.

## Release publication

By default, the workflow verifies builds and packaging without changing any release. Archives/checksums remain available as workflow artifacts. Set the boolean `publish_release` input to `true` only when intentionally publishing to the selected existing prerelease; the publish job runs after all selected native build jobs pass.

To verify both platforms without publishing:

```sh
gh workflow run portable-release.yml --repo AmedeoSaadMichel/easycat --ref main -f platform=both -f publish_release=false
``` The workflow has read-only repository access during building; only the publish job has contents-write permission. UI preview artifacts are kept separately and are not uploaded as application downloads.

These builds are unsigned test previews. CI validates compilation and engine workflows, and Xvfb validates Linux startup/rendering; it does not replace hands-on testing on Windows 10/11 or a range of Linux desktops.

The Linux archive also bundles libxkbcommon-x11 and its non-glibc dependency closure so X11 keyboard initialization does not depend on a separate development package. Package copyright notices and hashes are recorded under Licenses. Every archive records its application source commit and workflow run in build-info.json.

The Windows release uses Rust's static CRT configuration. A PE import-table check rejects executables that still require external VCRUNTIME/MSVCP DLLs. The workflow platform input supports rebuilding one platform independently after both have been initially validated.
