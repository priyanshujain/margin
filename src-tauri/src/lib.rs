mod epub;
mod library;
mod pdf;
mod project;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            epub::package_epub,
            pdf::compile_pdf,
            project::read_file,
            project::write_file,
            project::write_bytes,
            library::list_books,
            library::load_book,
            library::save_book,
            library::delete_book
        ])
        .run(tauri::generate_context!())
        .expect("error while running margin");
}
