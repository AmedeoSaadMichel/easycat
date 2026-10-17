# Third-party source and build information

These archives contain third-party code, not the easycat UI or engine. Source collection does not claim byte-for-byte reproducibility of upstream binaries. Keep the accompanying license, provenance and build-recipe files intact.

## macOS Apple Silicon

The macOS source bundle contains the 14 upstream archives/revisions listed in `macos-source-results.json`. Each non-Git archive is checked against the SHA-256 from the Homebrew formula saved in the original package. x264 is exported from the exact formula Git revision. `homebrew-recipes-and-notices/` contains the original formulae (including any inline DATA patches), receipts, SPDX SBOMs and license files.

Build dependency versions from these archives using the configure/CMake/Meson instructions in the saved formula `install` methods; apply any formula patches before configuring. These formulae also identify build tools and macOS SDK requirements. Build FFmpeg 9.0.2 with the arguments recorded in `macos-ffmpeg-buildconf.txt`, adjusting the installation prefix to your dependency prefix. Preserve `--enable-gpl` and `--enable-version3`; the resulting FFmpeg is GPLv3-or-later.

The easycat packaging step does not modify FFmpeg C/C++ source. It copies ffmpeg/ffprobe/ffplay and the recursive non-system Mach-O dependency closure. For each non-system load command, it runs `install_name_tool -change ORIGINAL_PATH @loader_path/RELATIVE_PATH_TO_FRAMEWORKS LIBRARY_OR_TOOL`. Dylib self-install names are changed with `install_name_tool -id @loader_path/FILENAME LIBRARY`. System `/usr/lib` and `/System` dependencies remain external. SDL3 is included for SDL2 compatibility's runtime loading. The package then ad-hoc signs the modified files with `codesign --force --sign - PATH`; Developer ID/notarization is a separate distribution step. These post-build relocations do not alter the upstream source versions.

## Windows x64 and Linux x86_64

The portable source bundle includes:

- FFmpeg commit `669f5ab083c7732c10f1ce9e4f1915960dd88457` (reported as `n9.0.2-28-g669f5ab083`).
- BtbN build scripts/patches at `63ac3265b0789ea16c7b1f8351cd2ad195a28722`, release `autobuild-2026-10-16-13-04`.
- A superset of 127 dependency source candidates from those recipes, including complete initialized submodule trees where present. SVN sources use LAME r6835 and Xvid r2204. The source result manifest records resolved revisions and checksums.
- The registry dependency sources from the original Cargo.lock files of rav1e and librsvg, with Cargo vendor configuration files. These are separate from easycat's Rust dependencies.

BtbN's `makeimage.sh`, `generate.sh`, `build.sh`, `scripts.d/`, `variants/`, `addins/` and patches contain the actual upstream build instructions. The relevant variants are `win64 gpl-shared 9.0` and `linux64 gpl-shared 9.0`. The exact configure strings are in the platform buildconf files.

Do not rebuild from the current moving release branch or Docker `latest` image and call it the corresponding source of these binaries. Use the archived dependency trees and recorded revisions instead of the recipes' network downloads. For FFmpeg, replace the upstream `build.sh` clone of the moving release branch with a copy of the supplied FFmpeg snapshot (mount that snapshot into its build container). Use the supplied Cargo vendor directories in place of registry downloads. The original scripts describe compiler/toolchain setup; rebuilding and verifying each native variant from this preserved source set remains part of release review.

The easycat portable packaging step copies upstream bin/lib files without modifying FFmpeg code. Linux adds the separately listed keyboard/X11 libraries, preserving their installed copyright notices. Windows's app executable is independent of libav*; these DLLs belong to the separate FFmpeg tools.

## easycat Rust libraries and fonts

`rust-third-party-sources.tar.gz` contains only the 415 public crates.io package archives identified in its index. It excludes workspace packages and private registries. The archive is a conservative superset of target/build dependencies, so not every crate is present in each executable. Original crate archives preserve their full sources, metadata and embedded font notices. The package also includes recovered upstream license files when a crate omitted them, and identifies four metadata-only declarations without inventing upstream copyright notices.

For MIT/Apache-2.0 or similar alternatives, a permissive option can be used; preserve any conjunctive font/Unicode notices too. No listed direct Rust dependency requires licensing the easycat workspace under GPL. This does not override third-party notices, grant rights to easycat's source, or resolve rights in unrelated user media.
