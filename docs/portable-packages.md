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

The publish job runs only after both native build jobs pass and uploads archives/checksums to the selected existing prerelease. The workflow has read-only repository access during building; only the publish job has contents-write permission. UI preview artifacts are kept separately and are not uploaded as application downloads.

These builds are unsigned test previews. CI validates compilation and engine workflows, and Xvfb validates Linux startup/rendering; it does not replace hands-on testing on Windows 10/11 or a range of Linux desktops.
