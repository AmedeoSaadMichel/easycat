# Third-party software in easycat

The easycat packages include FFmpeg, ffprobe and ffplay, Copyright (c) 2000–2026 the FFmpeg developers. The FFmpeg builds distributed with easycat 0.1.0-preview.1 are licensed under **GNU GPL version 3 or later**, not LGPL-only. Their configurations enable GPL components including x264 and x265; `--enable-nonfree` was not found in the three audited configurations.

FFmpeg is free software: you may redistribute and modify it under its license. It is distributed WITHOUT ANY WARRANTY, including the implied warranties of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. The complete GPLv3 text accompanies these notices.

FFmpeg and its codec libraries are separate programs from easycat. easycat invokes the tools using command-line arguments, files and standard process pipes; it does not link against libavcodec, libavformat or the other FFmpeg libraries. The easycat application source is not part of the third-party source archives. The licenses of third-party components do not grant rights to unrelated easycat code or artwork, and any terms applicable to easycat must not restrict rights granted for those components.

Other bundled components retain their own copyrights and licenses. The macOS dependency set includes x264, x265, LAME, mpg123, dav1d, libvmaf, libvpx, OpenSSL, Opus, SDL2 compatibility, SDL3, SVT-AV1 and XZ. Windows/Linux FFmpeg builds include a broader set recorded in the upstream build recipes. Linux also includes separately bundled X11/keyboard support libraries. Their notices, build manifests and available source materials must accompany distribution.

The app includes Rust libraries and fonts with separate licenses. Generated `rust-index.json` and `rust-licenses/` preserve registry dependency license metadata and accompanying notice files; complete third-party crate archives are provided separately. Font license texts are included with the package.

## Obtaining corresponding source

For a published release, download the matching third-party source archive and checksums from the **same public release page as the application**. It must contain the precise source revisions, upstream/local patches and build recipes for the bundled copyleft components. A link to upstream HEAD, a recipe list or the easycat repository alone is not a substitute for corresponding source. Do not publish a release until that archive is complete and anonymously available. The preparation report currently distinguishes collected material from unresolved source requirements.

These statements describe third-party rights and the intended distribution structure; they are not a claim that an incomplete preparation bundle is ready for public redistribution.

Upstream information: https://ffmpeg.org/legal.html
