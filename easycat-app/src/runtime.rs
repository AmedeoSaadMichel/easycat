// File: easycat-app/src/runtime.rs
//! Locate packaged tools when launched outside a terminal.
use std::{env, path::Path};

fn helpers_for(executable: &Path) -> Option<std::path::PathBuf> {
    let helpers = executable.parent()?.join("Helpers");
    let suffix = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    ["ffmpeg", "ffprobe", "ffplay"]
        .iter()
        .all(|name| helpers.join(format!("{name}{suffix}")).is_file())
        .then_some(helpers)
}

pub fn configure_packaged_tools() {
    let Ok(executable) = env::current_exe() else {
        return;
    };
    let Some(helpers) = helpers_for(&executable) else {
        return;
    };
    let mut paths = vec![helpers];
    paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    if let Ok(path) = env::join_paths(paths) {
        env::set_var("PATH", path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packaged_tools_require_the_complete_toolset() {
        let directory = env::temp_dir().join(format!("easycat-tools-{}", std::process::id()));
        let helpers = directory.join("Helpers");
        std::fs::create_dir_all(&helpers).unwrap();
        let executable = directory.join("easycat");
        assert!(helpers_for(&executable).is_none());
        let suffix = if cfg!(target_os = "windows") {
            ".exe"
        } else {
            ""
        };
        for name in ["ffmpeg", "ffprobe", "ffplay"] {
            std::fs::write(helpers.join(format!("{name}{suffix}")), []).unwrap();
        }
        assert_eq!(helpers_for(&executable), Some(helpers));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
