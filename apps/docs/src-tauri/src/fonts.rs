#[tauri::command(async)]
pub fn fonts_list_system() -> Vec<String> {
    margin_typeset::fonts::system_families()
}
