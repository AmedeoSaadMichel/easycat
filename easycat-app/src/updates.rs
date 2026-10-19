// File: easycat-app/src/updates.rs
//! Public release policy, checked off the UI thread; failures always allow local use.
mod transport;
mod ui;

use eframe::egui;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};

/// Monotonic application build; increment for every newly distributed app build.
pub const BUILD: u64 = 5;
pub const VERSION: &str = "0.1.0-preview.5";
const REPOSITORY: &str = "https://github.com/AmedeoSaadMichel/easycat-releases";
const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/AmedeoSaadMichel/easycat-releases/main/updates.json";
const MAX_BYTES: usize = 65_536;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Release {
    pub latest_build: u64,
    pub minimum_build: u64,
    pub version: String,
}

impl Release {
    pub fn url(&self) -> String {
        format!("{REPOSITORY}/releases/tag/v{}", self.version)
    }
}

#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    platforms: HashMap<String, Release>,
}

fn platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "macos-arm64",
        ("windows", "x86_64") => "windows-x64",
        ("linux", "x86_64") => "linux-x86_64",
        _ => "unsupported",
    }
}

fn parse(bytes: &[u8], platform: &str) -> Result<Release, String> {
    if bytes.len() > MAX_BYTES {
        return Err("Update manifest exceeds size limit".into());
    }
    let mut manifest: Manifest = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if manifest.schema_version != 1 {
        return Err("Unsupported update schema".into());
    }
    let release = manifest
        .platforms
        .remove(platform)
        .ok_or("Unsupported platform")?;
    if release.minimum_build == 0
        || release.latest_build < release.minimum_build
        || release.version.is_empty()
        || release.version.len() > 64
        || !release.version.starts_with(|c: char| c.is_ascii_digit())
        || !release
            .version
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c))
    {
        return Err("Invalid update policy".into());
    }
    Ok(release)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Status {
    #[default]
    Unchecked,
    Current,
    Available(Release),
    Required(Release),
    Unavailable,
}

fn evaluate(result: Result<Release, String>, build: u64) -> Status {
    match result {
        Ok(release) if build < release.minimum_build => Status::Required(release),
        Ok(release) if build < release.latest_build => Status::Available(release),
        Ok(_) => Status::Current,
        Err(_) => Status::Unavailable,
    }
}

#[derive(Default)]
pub struct Updates {
    pub status: Status,
    receiver: Option<Receiver<Result<Release, String>>>,
    dismissed: bool,
    #[cfg(debug_assertions)]
    simulation: Option<bool>,
}

impl Updates {
    pub fn simulated(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            self.simulation.is_some()
        }
        #[cfg(not(debug_assertions))]
        {
            false
        }
    }

    #[cfg(debug_assertions)]
    pub fn simulate(&mut self, required: bool) {
        self.simulation = Some(required);
        self.receiver = None;
        self.dismissed = false;
        self.status = evaluate(
            Ok(Release {
                latest_build: BUILD + 1,
                minimum_build: if required { BUILD + 1 } else { BUILD },
                version: format!("0.1.0-preview.{}", BUILD + 1),
            }),
            BUILD,
        );
    }

    pub fn check(&mut self, ctx: &egui::Context) {
        #[cfg(debug_assertions)]
        if let Some(required) = self.simulation {
            self.simulate(required);
            return;
        }
        if self.receiver.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = transport::fetch().and_then(|bytes| parse(&bytes, platform()));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    pub fn poll(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err("Update worker unavailable".into()),
        };
        self.status = evaluate(result, BUILD);
        self.receiver = None;
        self.dismissed = false;
    }

    pub fn required(&self) -> bool {
        matches!(self.status, Status::Required(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> Release {
        Release {
            latest_build: 8,
            minimum_build: 5,
            version: "0.1.0-preview.8".into(),
        }
    }

    #[test]
    fn minimum_boundary_and_latest_build_are_distinct() {
        let r = release();
        assert_eq!(evaluate(Ok(r.clone()), 4), Status::Required(r.clone()));
        assert_eq!(evaluate(Ok(r.clone()), 5), Status::Available(r.clone()));
        assert_eq!(evaluate(Ok(r.clone()), 8), Status::Current);
        assert_eq!(evaluate(Ok(r), 9), Status::Current);
        assert_eq!(evaluate(Err("offline".into()), 1), Status::Unavailable);
    }

    #[test]
    fn policy_is_platform_specific_and_links_cannot_escape_the_release_repository() {
        let manifest = br#"{"schema_version":1,"platforms":{"windows-x64":{"latest_build":8,"minimum_build":5,"version":"0.1.0-preview.8"},"linux-x86_64":{"latest_build":4,"minimum_build":1,"version":"0.1.0-preview.4"}}}"#;
        assert_eq!(parse(manifest, "windows-x64").unwrap(), release());
        assert_eq!(parse(manifest, "linux-x86_64").unwrap().minimum_build, 1);
        assert!(parse(manifest, "macos-arm64").is_err());
        assert_eq!(
            release().url(),
            format!("{REPOSITORY}/releases/tag/v0.1.0-preview.8")
        );
        for version in ["https://example.com", "../../other", "", "1.0\nspoof"] {
            let bad = serde_json::json!({"schema_version":1,"platforms":{"linux-x86_64":{"latest_build":8,"minimum_build":5,"version":version}}});
            assert!(parse(&serde_json::to_vec(&bad).unwrap(), "linux-x86_64").is_err());
        }
        assert!(parse(&vec![b' '; MAX_BYTES + 1], "linux-x86_64").is_err());
        for bad in [
            b"not json".as_slice(),
            br#"{"schema_version":2,"platforms":{}}"#,
        ] {
            assert!(parse(bad, "linux-x86_64").is_err());
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn simulations_keep_retries_local_and_distinguish_optional_from_required() {
        let ctx = egui::Context::default();
        let mut updates = Updates::default();
        for required in [false, true] {
            updates.simulate(required);
            assert!(updates.simulated());
            assert_eq!(updates.required(), required);
            assert!(matches!(
                updates.status,
                Status::Available(_) | Status::Required(_)
            ));
            updates.check(&ctx);
            assert!(updates.receiver.is_none());
            assert_eq!(updates.required(), required);
        }
    }

    #[test]
    fn maintained_public_policy_is_valid_for_all_supported_platforms() {
        let bytes = include_bytes!("../../distribution/updates.json");
        for platform in ["macos-arm64", "windows-x64", "linux-x86_64"] {
            let release = parse(bytes, platform).unwrap();
            assert!(release.latest_build >= release.minimum_build);
        }
        for (latest, minimum) in [(3, 4), (3, 0)] {
            let bad = serde_json::json!({"schema_version":1,"platforms":{"linux-x86_64":{"latest_build":latest,"minimum_build":minimum,"version":"0.1.0-preview.3"}}});
            assert!(parse(&serde_json::to_vec(&bad).unwrap(), "linux-x86_64").is_err());
        }
    }

    #[test]
    fn failed_retry_removes_a_previous_block_but_pending_retry_does_not() {
        let (tx, rx) = mpsc::channel();
        let mut updates = Updates {
            status: Status::Required(release()),
            receiver: Some(rx),
            dismissed: false,
            #[cfg(debug_assertions)]
            simulation: None,
        };
        updates.poll();
        assert!(updates.required());
        tx.send(Err("offline".into())).unwrap();
        updates.poll();
        assert!(!updates.required());
        assert_eq!(updates.status, Status::Unavailable);
    }
}
