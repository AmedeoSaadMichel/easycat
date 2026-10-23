// File: easycat-core/examples/bench_8bit.rs
//! Benchmark VideoToolbox without fallback using generated clips only.
//! Run with `cargo run --release -p easycat-core --example bench_8bit`.
use easycat_core::{
    ffmpeg::{build_args, Backend},
    probe, Codec, Container, Job, Mode, RateControl, Settings, Speed,
};
use std::{path::Path, process::Command, time::Instant};

fn execute(args: &[String]) -> Result<f64, Box<dyn std::error::Error>> {
    let start = Instant::now();
    let out = Command::new("ffmpeg").args(args).output()?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned().into());
    }
    Ok(start.elapsed().as_secs_f64())
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("easycat-bench-8bit-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let cpu = Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()?;
    let version = Command::new("ffmpeg").arg("-version").output()?;
    let date = Command::new("date").args(["+%Y-%m-%d"]).output()?;
    let mut results = Vec::new();
    for (size, width, height) in [("1080p", 1920, 1080), ("4K", 3840, 2160)] {
        for (input_codec, encoder) in [("h264", "h264_videotoolbox"), ("hevc", "hevc_videotoolbox")]
        {
            let input = dir.join(format!("{size}-{input_codec}.mp4"));
            let mut generate: Vec<String> = ["-v", "error", "-nostdin", "-y", "-f", "lavfi", "-i"]
                .map(String::from)
                .to_vec();
            generate.extend([
                format!("testsrc2=size={width}x{height}:rate=30"),
                "-t".into(),
                "8".into(),
                "-pix_fmt".into(),
                "nv12".into(),
                "-c:v".into(),
                encoder.into(),
                "-b:v".into(),
                "20M".into(),
                input.display().to_string(),
            ]);
            execute(&generate)?;
            let info = probe(&input)?;
            if info.bit_depth != 8 {
                return Err("source is not 8-bit".into());
            }
            for codec in [Codec::H264, Codec::H265] {
                for target in [None, Some(720)] {
                    let job = Job {
                        input: input.clone(),
                        output: dir.join("out.mp4"),
                        trim: None,
                        mode: Mode::Transcode(Settings {
                            crop: None,
                            codec,
                            container: Container::Mp4,
                            height: target,
                            rate: RateControl::Crf(23),
                            hardware: true,
                            speed: Speed::Balanced,
                        }),
                    };
                    let backends = [Backend::HardwareEncode, Backend::Chip];
                    let commands = backends
                        .map(|backend| build_args(&job, &info, backend))
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?;
                    let mut times = [Vec::new(), Vec::new()];
                    let mut bytes = [0, 0];
                    // One warm-up per backend; alternate five measurements to limit order bias.
                    for args in &commands {
                        execute(args)?;
                    }
                    for round in 0..5 {
                        for idx in if round % 2 == 0 { [0, 1] } else { [1, 0] } {
                            times[idx].push(execute(&commands[idx])?);
                            let output = probe(&job.output)?;
                            let expected_h = target.unwrap_or(height);
                            let expected_w = width * expected_h / height;
                            if (output.width, output.height, output.bit_depth)
                                != (expected_w, expected_h, 8)
                                || (output.duration - 8.0).abs() > 0.1
                            {
                                return Err(
                                    "unexpected output dimensions, duration or bit depth".into()
                                );
                            }
                            bytes[idx] = std::fs::metadata(&job.output)?.len();
                        }
                    }
                    let hardware = median(&mut times[0]);
                    let chip = median(&mut times[1]);
                    println!("{size} {input_codec} → {} {}: encode-only {hardware:.3}s, full hardware {chip:.3}s ({:.2}x)", codec.label(), target.map_or("original".into(), |h| format!("{h}p")), hardware / chip);
                    results.push(serde_json::json!({"source_size":size,"source_codec":input_codec,"output_codec":codec.label(),"output_height":target.unwrap_or(height),"hardware_seconds":times[0],"chip_seconds":times[1],"hardware_median":hardware,"chip_median":chip,"speedup":hardware/chip,"output_bytes":bytes}));
                }
            }
        }
    }
    let report = serde_json::json!({"date":String::from_utf8_lossy(&date.stdout).trim(),"cpu":String::from_utf8_lossy(&cpu.stdout).trim(),"ffmpeg":String::from_utf8_lossy(&version.stdout).lines().next(),"seconds":8,"fps":30,"repetitions":5,"warmups":1,"results":results});
    let output = Path::new("docs/benchmarks/8bit-videotoolbox.json");
    std::fs::write(output, serde_json::to_string_pretty(&report)?)?;
    println!("Results: {}", output.display());
    std::fs::remove_dir_all(dir)?;
    Ok(())
}
