use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Deserialize, Serialize)]
pub struct EpubFile {
    path: String,
    data: String,
    encoding: String,
}

fn is_text(path: &str) -> bool {
    let lower = path.to_lowercase();
    [
        ".opf", ".ncx", ".xhtml", ".html", ".htm", ".xml", ".css", ".txt",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
        || lower.ends_with("mimetype")
}

fn build(files: &[EpubFile]) -> Result<Vec<u8>, String> {
    let buffer = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buffer);

    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file("mimetype", stored).map_err(|e| e.to_string())?;
    zip.write_all(b"application/epub+zip").map_err(|e| e.to_string())?;

    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for file in files {
        if file.path == "mimetype" {
            continue;
        }
        let bytes = match file.encoding.as_str() {
            "base64" => STANDARD
                .decode(file.data.as_bytes())
                .map_err(|e| format!("failed to decode \"{}\": {e}", file.path))?,
            _ => file.data.as_bytes().to_vec(),
        };
        zip.start_file(&file.path, deflated).map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }

    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

#[tauri::command(async)]
pub fn package_epub(files: Vec<EpubFile>) -> Result<tauri::ipc::Response, String> {
    build(&files).map(tauri::ipc::Response::new)
}

fn unzip(bytes: &[u8]) -> Result<Vec<EpubFile>, String> {
    let mut archive = ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let path = entry.name().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        let (data, encoding) = if is_text(&path) {
            match String::from_utf8(buf) {
                Ok(text) => (text, "utf8"),
                Err(err) => (STANDARD.encode(err.into_bytes()), "base64"),
            }
        } else {
            (STANDARD.encode(&buf), "base64")
        };
        files.push(EpubFile {
            path,
            data,
            encoding: encoding.to_string(),
        });
    }
    Ok(files)
}

#[tauri::command]
pub fn unzip_epub(path: String) -> Result<Vec<EpubFile>, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("failed to read \"{path}\": {e}"))?;
    unzip(&bytes)
}
