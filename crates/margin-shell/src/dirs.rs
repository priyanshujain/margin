use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tauri::{Manager, Runtime};

/// The app's own data directory, created if missing. Exactly what every app used before, so no
/// existing data moves.
pub fn app_data_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// The directory shared by the suite, for suite-wide configuration only. On Apple platforms the
/// App Group container (`None` when the build lacks the entitlement); on Linux and Windows
/// `<platform data dir>/studio.margin`; `None` on Android, which has no shared location.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn shared_dir(group: &str) -> Option<PathBuf> {
    use objc2_foundation::{NSFileManager, NSString};
    let manager = NSFileManager::defaultManager();
    let url =
        manager.containerURLForSecurityApplicationGroupIdentifier(&NSString::from_str(group))?;
    let path = url.path()?;
    Some(PathBuf::from(path.to_string()))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn shared_dir(_group: &str) -> Option<PathBuf> {
    platform_data_dir().map(|dir| dir.join(SHARED_NAME))
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "linux",
    target_os = "windows"
)))]
pub fn shared_dir(_group: &str) -> Option<PathBuf> {
    None
}

#[cfg(any(target_os = "linux", target_os = "windows", test))]
const SHARED_NAME: &str = "studio.margin";

#[cfg(target_os = "linux")]
fn platform_data_dir() -> Option<PathBuf> {
    data_dir_from(std::env::var_os("XDG_DATA_HOME"), std::env::var_os("HOME"))
}

#[cfg(target_os = "windows")]
fn platform_data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
}

/// `$XDG_DATA_HOME` when it is an absolute path, else `$HOME/.local/share`, as the XDG spec says.
#[cfg(any(target_os = "linux", test))]
fn data_dir_from(
    xdg_data_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    xdg_data_home
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| {
            home.filter(|home| !home.is_empty())
                .map(|home| PathBuf::from(home).join(".local/share"))
        })
}

/// Write through a sibling temporary file and rename over the target, so a crash mid-write never
/// leaves a truncated file.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().ok_or("that path has no directory")?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let temp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    {
        let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&temp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_linux_data_dir_follows_xdg_and_falls_back_to_home() {
        let data = |xdg: Option<&str>, home: Option<&str>| {
            data_dir_from(xdg.map(Into::into), home.map(Into::into))
                .map(|dir| dir.join(SHARED_NAME))
        };
        assert_eq!(
            data(Some("/x/data"), Some("/home/a")),
            Some(PathBuf::from("/x/data/studio.margin"))
        );
        assert_eq!(
            data(Some("relative"), Some("/home/a")),
            Some(PathBuf::from("/home/a/.local/share/studio.margin"))
        );
        assert_eq!(
            data(None, Some("/home/a")),
            Some(PathBuf::from("/home/a/.local/share/studio.margin"))
        );
        assert_eq!(data(None, None), None);
    }

    #[test]
    fn atomic_write_replaces_the_file_and_leaves_no_temporary() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let path = dir.path().join("settings.json");

        atomic_write(&path, b"{\"one\":1}").expect("first write");
        atomic_write(&path, b"{\"two\":2}").expect("second write");

        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"two\":2}");
        let left: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(left, vec!["settings.json".to_string()]);
    }
}
