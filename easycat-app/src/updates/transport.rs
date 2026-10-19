// File: easycat-app/src/updates/transport.rs
//! Use the OS HTTPS client without adding a bundled networking dependency.
use super::{MANIFEST_URL, MAX_BYTES};
use std::io::Read;
use std::process::{Command, Stdio};

fn client() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::path::PathBuf::from(
            std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()),
        )
        .join("System32/curl.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::path::PathBuf::from("/usr/bin/curl")
    }
}

fn command() -> Command {
    let mut command = Command::new(client());
    command.args([
        "--disable",
        "--proto",
        "=https",
        "--tlsv1.2",
        "--fail",
        "--silent",
        "--show-error",
        "--connect-timeout",
        "5",
        "--max-time",
        "12",
        "--max-filesize",
        "65536",
        "--header",
        "Cache-Control: no-cache",
        MANIFEST_URL,
    ]);
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    command
}

pub(super) fn fetch() -> Result<Vec<u8>, String> {
    let mut child = command().spawn().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    let read = match child.stdout.take() {
        Some(output) => output.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes),
        None => Err(std::io::Error::other("Missing update response")),
    };
    if read.is_err() || bytes.len() > MAX_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Invalid update response".into());
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Update service unavailable".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_request_is_fixed_https_bounded_and_ignores_local_curl_configuration() {
        let cmd = command();
        let args: Vec<_> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "--disable");
        assert_eq!(args.last().unwrap(), MANIFEST_URL);
        assert!(args.windows(2).any(|v| v == ["--max-time", "12"]));
        assert!(args.windows(2).any(|v| v == ["--proto", "=https"]));
        assert!(!args.iter().any(|a| a == "--insecure" || a == "--location"));
    }
}
