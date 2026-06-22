use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(serde::Serialize)]
pub struct BookSummary {
    id: String,
    title: String,
    author: String,
    corrupt: bool,
}

fn corrupt_summary(stem: &str) -> BookSummary {
    BookSummary {
        id: stem.to_string(),
        title: "Unreadable book".to_string(),
        author: String::new(),
        corrupt: true,
    }
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
        books.push(BookSummary {
            id: id.to_string(),
            title,
            author,
            corrupt: false,
        });
    }
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
