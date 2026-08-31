use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::Manager;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSummary {
    id: String,
    title: String,
    author: String,
    corrupt: bool,
    updated_at: u64,
}

fn corrupt_summary(stem: &str) -> BookSummary {
    BookSummary {
        id: stem.to_string(),
        title: "Unreadable project".to_string(),
        author: String::new(),
        corrupt: true,
        updated_at: 0,
    }
}

fn file_modified_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn latest_chapter_ms(value: &serde_json::Value) -> u64 {
    value
        .get("chapters")
        .and_then(|c| c.as_array())
        .map(|chapters| {
            chapters
                .iter()
                .filter_map(|c| c.get("updatedAt").and_then(|v| v.as_u64()))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

pub(crate) fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub(crate) fn library_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("library");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn book_path(app: &tauri::AppHandle, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("invalid book id".to_string());
    }
    Ok(library_dir(app)?.join(format!("{id}.margin")))
}

#[tauri::command]
pub fn list_books(app: tauri::AppHandle) -> Result<Vec<BookSummary>, String> {
    let dir = library_dir(&app)?;
    let mut books = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(_) => continue,
        };
        if path.extension().and_then(|e| e.to_str()) != Some("margin") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(_) => {
                books.push(corrupt_summary(stem));
                continue;
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(value) => value,
            Err(_) => {
                books.push(corrupt_summary(stem));
                continue;
            }
        };
        let id = value.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() {
            books.push(corrupt_summary(stem));
            continue;
        }
        let metadata = value.get("metadata");
        let title = metadata
            .and_then(|m| m.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();
        let author = metadata
            .and_then(|m| m.get("author"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let updated_at = match latest_chapter_ms(&value) {
            0 => file_modified_ms(&path),
            ms => ms,
        };
        books.push(BookSummary {
            id: id.to_string(),
            title,
            author,
            corrupt: false,
            updated_at,
        });
    }
    books.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
    Ok(books)
}

#[tauri::command]
pub fn load_book(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let path = book_path(&app, &id)?;
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_book(app: tauri::AppHandle, id: String, contents: String) -> Result<(), String> {
    let path = book_path(&app, &id)?;
    crate::project::atomic_write(&path, contents.as_bytes(), true)
}

#[tauri::command]
pub fn delete_book(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let path = book_path(&app, &id)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
