//! Per-user pause shared by the WinRust shell components.
use std::{env, fs, io, path::{Path, PathBuf}};

fn marker() -> io::Result<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .filter(|dir| !dir.is_empty())
        .map(|dir| PathBuf::from(dir).join("WinRust"))
        .map(|dir| dir.join("windows-mode"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA"))
}

pub fn enabled() -> bool {
    marker().is_ok_and(|path| path.is_file())
}

pub fn set_enabled(enabled: bool) -> io::Result<()> {
    set_marker(&marker()?, enabled)
}

fn set_marker(path: &Path, enabled: bool) -> io::Result<()> {
    if enabled {
        if let Some(dir) = path.parent() { fs::create_dir_all(dir)?; }
        return fs::File::create(path).map(drop);
    }
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

pub fn explorer() -> PathBuf {
    PathBuf::from(env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into())).join("explorer.exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pausing_and_resuming_are_repeatable() {
        let prefs = env::temp_dir().join(format!("winrust-windows-mode-{}", std::process::id()));
        let marker = prefs.join("windows-mode");
        set_marker(&marker, true).unwrap();
        set_marker(&marker, true).unwrap();
        fs::write(prefs.join("style.txt"), "modern").unwrap();
        assert!(marker.is_file());

        set_marker(&marker, false).unwrap();
        set_marker(&marker, false).unwrap();
        assert!(!marker.exists());
        assert_eq!(fs::read_to_string(prefs.join("style.txt")).unwrap(), "modern");
        fs::remove_dir_all(prefs).unwrap();
    }
}
