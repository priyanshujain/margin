use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

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
    margin_shell::dirs::app_data_dir(app)
}

pub(crate) fn library_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    library_in(&app_data_dir(app)?)
}

pub fn library_in(data_dir: &Path) -> Result<PathBuf, String> {
    let dir = data_dir.join("library");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn book_path_in(library: &Path, id: &str) -> Result<PathBuf, String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("invalid project id".to_string());
    }
    Ok(library.join(format!("{id}.margin")))
}

fn book_path(app: &tauri::AppHandle, id: &str) -> Result<PathBuf, String> {
    book_path_in(&library_dir(app)?, id)
}

#[tauri::command]
pub fn list_books(app: tauri::AppHandle) -> Result<Vec<BookSummary>, String> {
    list_books_in(&library_dir(&app)?)
}

pub fn list_books_in(dir: &Path) -> Result<Vec<BookSummary>, String> {
    let mut books = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn book(id: &str, title: &str, updated: u64) -> String {
        format!(
            r#"{{"id":"{id}","metadata":{{"title":"{title}","author":"A"}},"chapters":[{{"id":"c1","updatedAt":{updated}}}]}}"#
        )
    }

    #[test]
    fn book_ids_cannot_leave_the_library() {
        let library = Path::new("/lib");
        assert!(book_path_in(library, "abc-123_X").is_ok());
        for bad in ["", "../escape", "a/b", "a.b", "a b", "..", "x\\y"] {
            assert!(book_path_in(library, bad).is_err(), "{bad} was accepted");
        }
    }

    #[test]
    fn lists_books_newest_first_and_flags_unreadable_ones() {
        let dir = tempfile::tempdir().unwrap();
        let library = library_in(dir.path()).unwrap();
        fs::write(library.join("old.margin"), book("old", "Old", 10)).unwrap();
        fs::write(library.join("new.margin"), book("new", "New", 20)).unwrap();
        fs::write(library.join("broken.margin"), "{not json").unwrap();
        fs::write(library.join("noid.margin"), "{}").unwrap();
        fs::write(library.join("notes.txt"), "ignored").unwrap();

        let books = list_books_in(&library).unwrap();
        let ids: Vec<_> = books.iter().map(|b| (b.id.as_str(), b.corrupt)).collect();
        assert_eq!(ids[..2], [("new", false), ("old", false)]);
        assert_eq!(books.len(), 4);
        assert!(
            books[2..]
                .iter()
                .all(|b| b.corrupt && b.title == "Unreadable project")
        );
        assert_eq!(books[0].title, "New");
        assert_eq!(books[0].author, "A");
    }

    #[test]
    fn saving_keeps_the_previous_copy_and_round_trips_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let library = library_in(dir.path()).unwrap();
        let path = book_path_in(&library, "b1").unwrap();
        let first = book("b1", "Ünïcode “quotes”", 1);
        let second = book("b1", "Second", 2);
        crate::project::atomic_write(&path, first.as_bytes(), true).unwrap();
        crate::project::atomic_write(&path, second.as_bytes(), true).unwrap();
        assert_eq!(fs::read(&path).unwrap(), second.as_bytes());
        assert_eq!(
            fs::read(library.join("b1.margin.bak")).unwrap(),
            first.as_bytes()
        );
        assert!(!library.join("b1.margin.tmp").exists());
    }
}
