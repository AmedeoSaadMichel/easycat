// File: easycut-app/build.rs
//! Embed the mascot icon into the Windows executable without extra crates.
fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.ico");
    #[cfg(target_os = "windows")]
    embed_windows_icon();
}

#[cfg(target_os = "windows")]
fn embed_windows_icon() {
    use std::{env, fs, path::PathBuf, process::Command};
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    let manifest =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets manifest directory"));
    let icon = manifest
        .join("assets/app-icon.ico")
        .display()
        .to_string()
        .replace('\\', "/");
    let rc = out.join("easycat.rc");
    let res = out.join("easycat.res");
    fs::write(&rc, format!("1 ICON \"{icon}\"\n")).expect("write icon resource");
    let kits =
        PathBuf::from(env::var_os("ProgramFiles(x86)").expect("Windows SDK program directory"))
            .join("Windows Kits/10/bin");
    let mut versions: Vec<_> = fs::read_dir(kits)
        .expect("Windows SDK installed")
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("x64/rc.exe"))
        .filter(|path| path.is_file())
        .collect();
    versions.sort();
    let compiler = versions.last().expect("Windows SDK resource compiler");
    let status = Command::new(compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res)
        .arg(&rc)
        .status()
        .expect("run Windows resource compiler");
    assert!(status.success(), "resource compiler failed");
    println!("cargo:rustc-link-arg={}", res.display());
}
