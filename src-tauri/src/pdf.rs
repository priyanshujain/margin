use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use tauri::Emitter;
use typst::diag::{Severity, SourceDiagnostic, Warned};
use typst::layout::PagedDocument;
use typst_as_lib::TypstEngine;

static LITERATA: &[u8] = include_bytes!("../../public/fonts/Literata-VF.ttf");
static LITERATA_ITALIC: &[u8] = include_bytes!("../../public/fonts/Literata-Italic-VF.ttf");
static HANKEN: &[u8] = include_bytes!("../../public/fonts/HankenGrotesk-VF.ttf");
static HANKEN_ITALIC: &[u8] = include_bytes!("../../public/fonts/HankenGrotesk-Italic-VF.ttf");

#[derive(Deserialize)]
pub struct ImageInput {
    path: String,
    data: String,
}

fn compile(source: String, images: &[ImageInput]) -> Result<(Vec<u8>, String), String> {
    let mut binaries: Vec<(&str, Vec<u8>)> = Vec::with_capacity(images.len());
    for image in images {
        let bytes = STANDARD
            .decode(image.data.as_bytes())
            .map_err(|e| format!("failed to decode image \"{}\": {e}", image.path))?;
        binaries.push((image.path.as_str(), bytes));
    }

    let engine = TypstEngine::builder()
        .main_file(source)
        .fonts([LITERATA, LITERATA_ITALIC, HANKEN, HANKEN_ITALIC])
        .with_static_file_resolver(binaries)
        .build();

    let Warned { output, warnings } = engine.compile();
    let document: PagedDocument = output.map_err(|e| format_diagnostics(&e))?;
    let bytes =
        typst_pdf::pdf(&document, &Default::default()).map_err(|d| format_source_diagnostics(&d))?;
    let warning_text = if warnings.is_empty() {
        String::new()
    } else {
        format_source_diagnostics(&warnings)
    };
    Ok((bytes, warning_text))
}

#[tauri::command(async)]
pub fn compile_pdf(
    app: tauri::AppHandle,
    source: String,
    images: Vec<ImageInput>,
    emit_warnings: bool,
) -> Result<tauri::ipc::Response, String> {
    let (bytes, warnings) = compile(source, &images)?;
    if emit_warnings && !warnings.is_empty() {
        app.emit("pdf-warnings", warnings).ok();
    }
    Ok(tauri::ipc::Response::new(bytes))
}

fn format_diagnostics(error: &typst_as_lib::TypstAsLibError) -> String {
    match error {
        typst_as_lib::TypstAsLibError::TypstSource(diagnostics) => format_source_diagnostics(diagnostics),
        other => other.to_string(),
    }
}

fn format_source_diagnostics(diagnostics: &[SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let kind = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            let mut message = format!("{kind}: {}", diagnostic.message);
            for hint in &diagnostic.hints {
                message.push_str(&format!("\n  hint: {hint}"));
            }
            message
        })
        .collect::<Vec<_>>()
        .join("\n")
}
