use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use tauri::Emitter;

#[derive(Deserialize)]
pub struct ImageInput {
    path: String,
    data: String,
}

pub fn compile(
    source: String,
    images: &[ImageInput],
    bundled_fonts: &[String],
    system_fonts: &[String],
) -> Result<(Vec<u8>, String), String> {
    let images = images
        .iter()
        .map(|image| margin_typeset::Image {
            path: image.path.clone(),
            bytes: STANDARD
                .decode(image.data.as_bytes())
                .map_err(|e| format!("failed to decode image \"{}\": {e}", image.path)),
        })
        .collect();
    let (bytes, warnings) = margin_typeset::compile(source, images, bundled_fonts, system_fonts)?;
    let text = warnings
        .iter()
        .map(|warning| match warning.count {
            0 | 1 => format!("warning: {}", warning.message),
            n => format!("warning: {} ({n} times)", warning.message),
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok((bytes, text))
}

#[tauri::command(async)]
pub fn compile_pdf(
    app: tauri::AppHandle,
    source: String,
    images: Vec<ImageInput>,
    emit_warnings: bool,
    bundled_fonts: Vec<String>,
    system_fonts: Vec<String>,
) -> Result<tauri::ipc::Response, String> {
    let (bytes, warnings) = compile(source, &images, &bundled_fonts, &system_fonts)?;
    if emit_warnings && !warnings.is_empty() {
        app.emit("pdf-warnings", warnings).ok();
    }
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command(async)]
pub fn list_system_fonts() -> Vec<String> {
    margin_typeset::fonts::system_families()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedded(pdf: &[u8], name: &str) -> bool {
        pdf.windows(name.len()).any(|w| w == name.as_bytes())
    }

    #[test]
    fn headings_and_bold_embed_their_static_weights() {
        let source = r#"#set text(font: "Literata")
#text(weight: "medium")[Heading] #strong[bold] #text(font: "Lora", weight: "bold")[lora]"#;
        let (pdf, _) = compile(source.to_string(), &[], &["lora".to_string()], &[]).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        for face in ["Literata-Medium", "Literata-Bold", "Lora-Bold"] {
            assert!(embedded(&pdf, face), "{face} is not embedded");
        }
    }
}
