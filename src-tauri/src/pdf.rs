use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use typst::diag::{Severity, SourceDiagnostic};
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

fn compile(source: String, images: &[ImageInput]) -> Result<Vec<u8>, String> {
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

    let document: PagedDocument = engine.compile().output.map_err(|e| format_diagnostics(&e))?;

    typst_pdf::pdf(&document, &Default::default()).map_err(|d| format_source_diagnostics(&d))
}

#[tauri::command]
pub fn compile_pdf(source: String, images: Vec<ImageInput>) -> Result<tauri::ipc::Response, String> {
    compile(source, &images).map(tauri::ipc::Response::new)
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

#[cfg(test)]
mod tests {
    use super::*;

    // A faithful single-chapter preview source produced by chapterToPdfInputs
    // (preamble #let macros + page numbering reset + opener + body).
    const CHAPTER_PREVIEW: &str = r##"#set document(title: "The Lighthouse", author: "")
#set page(
  width: 6in + 0.125in,
  height: 9in + 0.125in * 2,
  margin: (inside: 0.875in, outside: 0.625in + 0.125in, top: 0.8in + 0.125in, bottom: 0.8in + 0.125in),
  binding: left,
)
#set text(font: "Literata", size: 11pt, lang: "en", hyphenate: true)
#set par(justify: true, leading: 0.72em, spacing: 0.72em, first-line-indent: (amount: 1.3em, all: false))
#show heading: set text(font: "Literata", weight: "medium")
#show heading.where(level: 1): set text(size: 22pt)

#let scenebreak = align(center)[#v(0.5em) #line(length: 13%, stroke: 0.5pt + rgb("#d6cfbd")) #v(0.5em)]
#let blockquote(body) = pad(left: 1.2em)[#set text(style: "italic", fill: rgb("#6b6458")); #body]
#let chapteropener(num, title) = {
  pagebreak(weak: true)
  v(2.1in)
  align(center)[
    #text(font: "Hanken Grotesk", size: 8.5pt, weight: "semibold", tracking: 0.28em)[#upper("Chapter " + num)]
    #v(0.7em)
    #heading(level: 1, numbering: none, outlined: true)[#title]
  ]
  v(1.5em)
}

#set page(numbering: "1")
#counter(page).update(1)

#chapteropener("1", [The Lighthouse])

The lamp had not been lit in thirty years.

#scenebreak

Morning came the color of pewter.
"##;

    #[test]
    fn compiles_a_single_chapter_preview() {
        let pdf = compile(CHAPTER_PREVIEW.to_string(), &[]).expect("chapter preview should compile");
        assert!(pdf.starts_with(b"%PDF"), "output should be a PDF");
    }
}
