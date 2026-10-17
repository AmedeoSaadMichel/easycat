# VideoToolbox benchmark for 8-bit video

Date: 2026-10-16. Machine: Apple M4. macOS 15.6 (24G84). FFmpeg 9.0.2.

## Conclusion

Do not automatically enable `Backend::Chip` for 8-bit sources based on this run. Native-resolution differences are small; scaling to 720p is consistently slower with the full hardware pipeline. Backend selection remains unchanged. This benchmark does not re-evaluate the existing policy for sources above 8 bit.

## Method

Synthetic testsrc2 clips: 8 seconds, 30 fps, no audio; 8-bit H.264 and HEVC sources at 1920×1080 and 3840×2160, generated with VideoToolbox, NV12, requested bitrate 20 Mbit/s. Outputs are MP4 H.264 or HEVC at source resolution or 720p, balanced speed, Crf(23), translated by the core to -q:v 54.

Commands come from easycut_core::ffmpeg::build_args. The harness forces HardwareEncode and Chip independently, without fallback. Each backend receives one warm-up and five timed runs in alternating order. Reported values are medians of whole-process wall time, including startup and file writing; source generation and probing are excluded. Every output is checked for dimensions, duration and 8-bit depth. Total: 16 combinations, 160 timed exports, 32 warm-ups.

Hardware measurements ran outside the sandbox because VideoToolbox cannot create a compression session inside it. VideoToolbox software fallback was not enabled.

## Median times

A positive delta means the full hardware pipeline is slower than hardware encoding alone.

| Source | Output | Encode only, s | Full hardware, s | Time delta |
|---|---|---:|---:|---:|
| 1080p h264 | H.264 1080p | 1.212 | 1.254 | +3.5% |
| 1080p h264 | H.264 720p | 0.679 | 0.860 | +26.6% |
| 1080p h264 | H.265 1080p | 1.283 | 1.309 | +2.0% |
| 1080p h264 | H.265 720p | 0.802 | 0.890 | +11.0% |
| 1080p hevc | H.264 1080p | 1.193 | 1.234 | +3.5% |
| 1080p hevc | H.264 720p | 0.707 | 0.752 | +6.3% |
| 1080p hevc | H.265 1080p | 1.276 | 1.310 | +2.7% |
| 1080p hevc | H.265 720p | 0.800 | 0.863 | +7.9% |
| 4K h264 | H.264 2160p | 4.161 | 4.192 | +0.8% |
| 4K h264 | H.264 720p | 0.756 | 2.039 | +169.5% |
| 4K h264 | H.265 2160p | 4.334 | 4.349 | +0.3% |
| 4K h264 | H.265 720p | 0.870 | 2.081 | +139.2% |
| 4K hevc | H.264 2160p | 4.183 | 4.166 | -0.4% |
| 4K hevc | H.264 720p | 0.948 | 1.951 | +105.9% |
| 4K hevc | H.265 2160p | 4.342 | 4.323 | -0.4% |
| 4K hevc | H.265 720p | 1.187 | 1.948 | +64.2% |


## Limitations

These results apply to this machine, FFmpeg build and synthetic clips. Energy consumption, perceptual quality, rotations, audio, HDR, other frame rates and long exports were not measured. No universal 8-bit or energy-efficiency conclusion follows. Scale and scale_vt can produce different images and output sizes; equal quality settings do not guarantee equal perceptual quality. The JSON contains each backend's five measurements and the last output size.

## Reproduce

From the repository root on macOS with VideoToolbox access:

```sh
cargo run --release -p easycut-core --example bench_8bit
```

The harness uses generated temporary clips, removes them after a successful run, and writes docs/benchmarks/8bit-videotoolbox.json. Copy that report before another run if you want to retain it. FFmpeg must include VideoToolbox encoders and scale_vt.
