use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use std::io::Write;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

#[derive(Deserialize)]
pub struct EpubFile {
    path: String,
    data: String,
    encoding: String,
}

fn build(files: &[EpubFile]) -> Result<Vec<u8>, String> {
    let buffer = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buffer);

    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file("mimetype", stored)
        .map_err(|e| e.to_string())?;
    zip.write_all(b"application/epub+zip")
        .map_err(|e| e.to_string())?;

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
        zip.start_file(&file.path, deflated)
            .map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }

    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

#[tauri::command]
pub fn package_epub(files: Vec<EpubFile>) -> Result<tauri::ipc::Response, String> {
    build(&files).map(tauri::ipc::Response::new)
}
