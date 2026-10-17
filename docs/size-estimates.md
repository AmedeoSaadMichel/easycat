# Output size estimate verification

The rough estimate uses dimensions, frame rate, codec, CRF and source bitrate. It cannot infer scene complexity. The UI now automatically measures the selected compression profile after a 700 ms debounce and labels the result as sample-based.

## Generated-clip checks

Software H.264 at CRF 28, balanced speed, MP4 output with audio. These measurements verify implementation behavior; they do not establish an accuracy guarantee for other content or hardware.

| Case | Rough estimate, bytes | Sample estimate, bytes | Actual export, bytes | Rough error | Sample error |
|---|---:|---:|---:|---:|---:|
| 12-second 1280×720 synthetic clip, whole-clip sample | 1,821,276 | 2,402,984 | 2,363,687 | 22.9% | 1.7% |
| 36-second selection from a 48-second 640×360 synthetic clip, five samples | 1,797,957 | 2,437,675 | 2,467,166 | 27.1% | 1.2% |

Reproduce with:

```sh
cargo test -p easycut-core --test integration -- --nocapture
```

The short-clip check requires error below 5%; the longer selection allows 15% to account for shorter GOPs from independent sample starts. Both must improve on the heuristic in their controlled fixtures.

## Other regression coverage

- Cancel in-progress FFmpeg samples promptly; abort mixed-backend measurements and retry the complete plan with the next backend.
- Invalidate pending and completed measurements when trim, hardware or speed changes.
- Debounce automatic measurement, measure only the selected profile, reuse cached results, and do not start measurement during export.
- Do not reserve audio bitrate for target-size exports when the source has no audio. Reserving 128 kbit/s incorrectly would undershoot a nine-minute video-only target by approximately 8.64 MB, before muxing overhead.

## Remaining uncertainty

Five six-second windows sample only part of a long video. Variable scenes, encoder startup/GOP behavior, container overhead and hardware availability may still affect the final size. Audio is projected at its configured encoding bitrate. Copy/remux estimates remain source-size projections and may vary with bitrate and keyframe boundaries. Requested target size remains a bitrate target, not an exact byte count.
