use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use tauri::{Emitter, Manager};

use crate::dto::{ImageInput, PdfWarning};

/// The bytes of one image, either the ones that came with the request or the ones on disk.
///
/// A read is guarded exactly as every read in fs.rs is: the path is resolved and has to land
/// inside a folder the user actually opened. A document is untrusted input, `![](../../../.ssh/id_rsa)`
/// is a link anybody can type, and an exporter is not where this app starts reading outside an
/// open folder. The path has to be absolute for that check to mean anything, which is why the
/// converter sends an absolute one for any image it has no bytes for.
fn image_bytes(image: &ImageInput, root_paths: &[String]) -> Result<Vec<u8>, String> {
    match &image.data {
        Some(data) => STANDARD
            .decode(data.as_bytes())
            .map_err(|e| format!("could not decode \"{}\": {e}", image.path)),
        None => {
            let path = crate::fs::resolve_in_roots(root_paths, &image.path)?;
            std::fs::read(&path).map_err(|e| format!("could not read \"{}\": {e}", image.path))
        }
    }
}

pub fn compile(
    source: String,
    images: &[ImageInput],
    root_paths: &[String],
) -> Result<(Vec<u8>, Vec<PdfWarning>), String> {
    compile_in(source, images, root_paths, &[], &[])
}

pub fn compile_in(
    source: String,
    images: &[ImageInput],
    root_paths: &[String],
    bundled: &[String],
    system: &[String],
) -> Result<(Vec<u8>, Vec<PdfWarning>), String> {
    let images = images
        .iter()
        .map(|image| margin_typeset::Image {
            path: image.path.clone(),
            bytes: image_bytes(image, root_paths),
        })
        .collect();
    margin_typeset::compile(source, images, bundled, system)
}

/// Compiles Typst source to PDF bytes. Warnings, when there are any, arrive separately on the
/// `pdf-warnings` event, because a compile answers with raw bytes and has nowhere to put a second
/// value.
#[tauri::command(async)]
pub fn pdf_compile(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    source: String,
    images: Vec<ImageInput>,
    bundled_fonts: Vec<String>,
    system_fonts: Vec<String>,
) -> Result<tauri::ipc::Response, String> {
    // The same gate `fs::checked` puts in front of every other read, reached the same way it is:
    // the paths this window may look at, then `resolve_in_roots`. The locks are dropped before the
    // compile, which is the slowest thing this app does and has no business holding them.
    let root_paths = app.state::<crate::windows::Windows>().allowed(
        &app.state::<crate::Roots>(),
        window.label(),
        crate::windows::Access::Nearby,
    )?;

    let (bytes, warnings) = compile_in(source, &images, &root_paths, &bundled_fonts, &system_fonts)?;
    if !warnings.is_empty() {
        app.emit_to(
            tauri::EventTarget::webview_window(window.label()),
            "pdf-warnings",
            warnings,
        )
        .ok();
    }
    Ok(tauri::ipc::Response::new(bytes))
}

/// Writes the finished file to wherever the save panel pointed.
#[tauri::command(async)]
pub fn pdf_write(path: String, bytes: Vec<u8>) -> Result<(), String> {
    // No root guard here, deliberately, and the next person to read this will assume that is a
    // bug. It is not: this path came from the user through a native save panel, so it is the
    // user's own choice of destination and not something a document said. The guard exists to stop
    // an untrusted document naming a path, and saving somewhere outside every open folder is the
    // ordinary case rather than the attack.
    //
    // What that argument does not cover is a destination that is already one of the user's
    // documents. `atomic_write` replaces whatever is there, so a `.md` name typed into the save
    // panel is the one way an export can destroy markdown, and this app does not get to be the
    // reason a document is gone. The save panel carries a PDF filter and AppKit usually appends
    // `.pdf` to a name typed with another extension, but that is the panel being careful rather
    // than this command, and the promise is not the panel's to keep.
    //
    // Only a file that is already there is refused. Writing a document-shaped name that nothing
    // holds yet is odd rather than destructive, and the user can see what they typed.
    let target = Path::new(&path);
    if target.is_file() && matches!(crate::fs::kind_for(target, false), "markdown" | "text") {
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        return Err(format!("{name} is a document. A PDF cannot be written over it."));
    }
    crate::fs::atomic_write(target, &bytes)
}
