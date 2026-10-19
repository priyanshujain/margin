pub use margin_mac::writing_tools::install;

#[tauri::command]
pub fn writing_available() -> Result<bool, String> {
    Ok(margin_mac::writing_tools::available())
}

#[tauri::command]
pub fn writing_run(app: tauri::AppHandle, tool: String) -> Result<(), String> {
    margin_mac::writing_tools::run(&app, tool)
}
