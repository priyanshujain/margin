use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

static LITERATA: &[u8] = include_bytes!("../../public/fonts/Literata-VF.ttf");
static LITERATA_ITALIC: &[u8] = include_bytes!("../../public/fonts/Literata-Italic-VF.ttf");

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

    for (path, bytes) in [
        ("OEBPS/fonts/Literata-VF.ttf", LITERATA),
        ("OEBPS/fonts/Literata-Italic-VF.ttf", LITERATA_ITALIC),
    ] {
        zip.start_file(path, deflated).map_err(|e| e.to_string())?;
        zip.write_all(bytes).map_err(|e| e.to_string())?;
    }

    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

#[tauri::command]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn text(path: &str, data: &str) -> EpubFile {
        EpubFile {
            path: path.into(),
            data: data.into(),
            encoding: "utf8".into(),
        }
    }

    #[test]
    fn round_trips_through_build_and_unzip() {
        let png = STANDARD.encode([0x89, 0x50, 0x4e, 0x47]);
        let input = vec![
            text("mimetype", "application/epub+zip"),
            text("META-INF/container.xml", "<container/>"),
            text("OEBPS/chapter-1.xhtml", "<html>café</html>"),
            EpubFile {
                path: "OEBPS/assets/figure-1.png".into(),
                data: png.clone(),
                encoding: "base64".into(),
            },
        ];

        let zipped = build(&input).expect("build");
        let out = unzip(&zipped).expect("unzip");
        let find = |p: &str| out.iter().find(|f| f.path == p).expect(p);

        assert_eq!(find("mimetype").data, "application/epub+zip");
        let chapter = find("OEBPS/chapter-1.xhtml");
        assert_eq!(chapter.encoding, "utf8");
        assert_eq!(chapter.data, "<html>café</html>");
        let figure = find("OEBPS/assets/figure-1.png");
        assert_eq!(figure.encoding, "base64");
        assert_eq!(figure.data, png);
        // Fonts are embedded by build() and must come back as base64 binary.
        assert_eq!(find("OEBPS/fonts/Literata-VF.ttf").encoding, "base64");
    }

    #[test]
    fn unzip_epub_reads_a_file_from_disk() {
        let zipped = build(&[
            text("mimetype", "application/epub+zip"),
            text("OEBPS/chapter-1.xhtml", "<p>hi</p>"),
        ])
        .expect("build");

        let mut path = std::env::temp_dir();
        path.push("margin-unzip-test.epub");
        std::fs::write(&path, &zipped).expect("write");

        let out = unzip_epub(path.to_string_lossy().to_string()).expect("unzip_epub");
        let _ = std::fs::remove_file(&path);

        let chapter = out
            .iter()
            .find(|f| f.path == "OEBPS/chapter-1.xhtml")
            .expect("chapter present");
        assert_eq!(chapter.data, "<p>hi</p>");
        assert_eq!(chapter.encoding, "utf8");
    }
}
