#[tauri::command]
pub fn writing_available() -> bool {
    margin_mac::writing_tools::available()
}

#[tauri::command]
pub fn run_writing_tool(app: tauri::AppHandle, tool: String) -> Result<(), String> {
    margin_mac::writing_tools::run(&app, tool)
}
