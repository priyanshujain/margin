use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use tauri_plugin_dialog::DialogExt;

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

pub fn atomic_write(path: &Path, bytes: &[u8], backup: bool) -> Result<(), String> {
    let tmp = with_suffix(path, ".tmp");
    {
        let mut f = File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    if path.exists() {
        if backup {
            let bak = with_suffix(path, ".bak");
            let _ = fs::remove_file(&bak);
            fs::rename(path, &bak).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[derive(Default)]
pub struct GrantedPaths(Mutex<HashSet<PathBuf>>);

#[derive(Deserialize)]
pub struct DialogFilter {
    name: String,
    extensions: Vec<String>,
}

fn normalise(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(format!("refusing path \"{}\"", path.display()));
    }
    let name = path
        .file_name()
        .ok_or_else(|| format!("refusing path \"{}\"", path.display()))?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("refusing path \"{}\"", path.display()))?;
    let resolved = fs::canonicalize(parent)
        .map_err(|e| e.to_string())?
        .join(name);
    match fs::canonicalize(&resolved) {
        Ok(target) => Ok(target),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(resolved),
        Err(e) => Err(e.to_string()),
    }
}

fn is_permitted(path: &Path, data_dir: &Path, granted: &HashSet<PathBuf>) -> bool {
    path.starts_with(data_dir) || granted.contains(path)
}

pub(crate) fn checked_path(
    app: &tauri::AppHandle,
    granted: &GrantedPaths,
    path: &str,
) -> Result<PathBuf, String> {
    checked_path_in(&crate::library::app_data_dir(app)?, granted, path)
}

pub fn checked_path_in(
    data_dir: &Path,
    granted: &GrantedPaths,
    path: &str,
) -> Result<PathBuf, String> {
    let resolved = normalise(Path::new(path))?;
    let data_dir = fs::canonicalize(data_dir).map_err(|e| e.to_string())?;
    let granted = granted.0.lock().map_err(|e| e.to_string())?;
    if is_permitted(&resolved, &data_dir, &granted) {
        Ok(resolved)
    } else {
        Err(format!("refusing path \"{path}\""))
    }
}

pub fn grant_path(granted: &GrantedPaths, path: &Path) -> Result<(), String> {
    granted
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(normalise(path)?);
    Ok(())
}

fn grant(
    granted: &GrantedPaths,
    picked: Option<tauri_plugin_dialog::FilePath>,
) -> Result<Option<String>, String> {
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    grant_path(granted, &path)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

fn with_filters<R: tauri::Runtime>(
    mut builder: tauri_plugin_dialog::FileDialogBuilder<R>,
    filters: &[DialogFilter],
) -> tauri_plugin_dialog::FileDialogBuilder<R> {
    for filter in filters {
        let extensions: Vec<&str> = filter.extensions.iter().map(String::as_str).collect();
        builder = builder.add_filter(&filter.name, &extensions);
    }
    builder
}

#[tauri::command]
pub async fn pick_save_path(
    window: tauri::WebviewWindow,
    granted: tauri::State<'_, GrantedPaths>,
    default_name: String,
    filters: Vec<DialogFilter>,
) -> Result<Option<String>, String> {
    let builder = with_filters(window.dialog().file(), &filters)
        .set_parent(&window)
        .set_file_name(default_name);
    grant(&granted, builder.blocking_save_file())
}

#[tauri::command]
pub async fn pick_open_path(
    window: tauri::WebviewWindow,
    granted: tauri::State<'_, GrantedPaths>,
    filters: Vec<DialogFilter>,
) -> Result<Option<String>, String> {
    let builder = with_filters(window.dialog().file(), &filters).set_parent(&window);
    grant(&granted, builder.blocking_pick_file())
}

#[tauri::command]
pub fn read_file(
    app: tauri::AppHandle,
    granted: tauri::State<'_, GrantedPaths>,
    path: String,
) -> Result<String, String> {
    fs::read_to_string(checked_path(&app, &granted, &path)?).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(
    app: tauri::AppHandle,
    granted: tauri::State<'_, GrantedPaths>,
    path: String,
    contents: String,
) -> Result<(), String> {
    atomic_write(
        &checked_path(&app, &granted, &path)?,
        contents.as_bytes(),
        true,
    )
}

#[tauri::command]
pub fn write_bytes(
    app: tauri::AppHandle,
    granted: tauri::State<'_, GrantedPaths>,
    path: String,
    data: String,
) -> Result<(), String> {
    let bytes = STANDARD
        .decode(data.as_bytes())
        .map_err(|e| e.to_string())?;
    atomic_write(&checked_path(&app, &granted, &path)?, &bytes, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let outside = root.path().join("outside");
        fs::create_dir_all(data.join("library")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        (root, data, outside)
    }

    #[test]
    fn paths_inside_the_data_dir_are_allowed() {
        let (_root, data, _) = setup();
        let granted = GrantedPaths::default();
        let path = data.join("library").join("a.margin");
        let checked = checked_path_in(&data, &granted, path.to_str().unwrap()).unwrap();
        assert!(checked.ends_with("library/a.margin"));
    }

    #[test]
    fn relative_dotted_and_outside_paths_are_refused() {
        let (_root, data, outside) = setup();
        let granted = GrantedPaths::default();
        let dotted = format!("{}/library/../../outside/x.txt", data.display());
        for path in [
            "relative.txt".to_string(),
            dotted,
            outside.join("x.txt").display().to_string(),
            "/etc/passwd".to_string(),
        ] {
            assert!(
                checked_path_in(&data, &granted, &path).is_err(),
                "{path} was allowed"
            );
        }
    }

    #[test]
    fn a_picked_file_outside_is_allowed_and_only_that_file() {
        let (_root, data, outside) = setup();
        let granted = GrantedPaths::default();
        let picked = outside.join("book.epub");
        grant_path(&granted, &picked).unwrap();
        assert!(checked_path_in(&data, &granted, picked.to_str().unwrap()).is_ok());
        let sibling = outside.join("other.epub");
        assert!(checked_path_in(&data, &granted, sibling.to_str().unwrap()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_the_data_dir_is_refused() {
        let (_root, data, outside) = setup();
        fs::write(outside.join("secret.txt"), "x").unwrap();
        std::os::unix::fs::symlink(outside.join("secret.txt"), data.join("link.txt")).unwrap();
        let granted = GrantedPaths::default();
        let link = data.join("link.txt");
        assert!(checked_path_in(&data, &granted, link.to_str().unwrap()).is_err());
    }
}
