use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

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

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_file(path: String, contents: String) -> Result<(), String> {
    atomic_write(Path::new(&path), contents.as_bytes(), true)
}

#[tauri::command]
pub fn write_bytes(path: String, data: String) -> Result<(), String> {
    let bytes = STANDARD.decode(data.as_bytes()).map_err(|e| e.to_string())?;
    atomic_write(Path::new(&path), &bytes, false)
}
